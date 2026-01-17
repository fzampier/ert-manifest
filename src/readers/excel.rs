use std::path::{Path, PathBuf};

use calamine::{open_workbook_auto, Data, Reader, Sheets};

use crate::inference::{is_missing, TypeInferencer};
use crate::privacy::{bucket_count, check_column_name, safe_count};
use crate::stats::ColumnStatTracker;
use crate::types::{
    Classification, ColumnSchema, ColumnStats, DType, ProcessingOptions, Result, SafeValue,
    SheetSchema, MAX_UNIQUE_VALUES,
};

use super::DataReader;

/// Excel file reader (supports .xlsx, .xls, .xlsm, .xlsb)
pub struct ExcelReader {
    path: PathBuf,
}

impl ExcelReader {
    pub fn new(path: &Path) -> Result<Self> {
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    /// Convert Excel Data to string representation
    fn data_to_string(dt: &Data) -> String {
        match dt {
            Data::Empty => String::new(),
            Data::String(s) => s.clone(),
            Data::Float(f) => f.to_string(),
            Data::Int(i) => i.to_string(),
            Data::Bool(b) => b.to_string(),
            Data::DateTime(d) => {
                // Convert ExcelDateTime to string using its as_f64 representation
                Self::excel_serial_to_date_string(d.as_f64())
            }
            Data::DateTimeIso(s) => s.clone(),
            Data::DurationIso(s) => s.clone(),
            Data::Error(e) => format!("#{:?}", e),
        }
    }

    /// Convert Excel serial date to ISO date string
    fn excel_serial_to_date_string(serial: f64) -> String {
        // Excel epoch is 1899-12-30 (with the 1900 leap year bug)
        let days = serial as i64;
        let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
        if let Some(date) = base.checked_add_signed(chrono::Duration::days(days)) {
            date.format("%Y-%m-%d").to_string()
        } else {
            serial.to_string()
        }
    }

    /// Check if a Data represents a missing value
    fn is_missing_data(dt: &Data) -> bool {
        match dt {
            Data::Empty => true,
            Data::String(s) => is_missing(s),
            Data::Error(_) => true,
            _ => false,
        }
    }

    /// Get numeric value from Data if applicable
    fn get_numeric_value(dt: &Data) -> Option<f64> {
        match dt {
            Data::Float(f) => Some(*f),
            Data::Int(i) => Some(*i as f64),
            Data::DateTime(d) => Some(d.as_f64()),
            _ => None,
        }
    }

    /// Infer type from Data
    fn infer_type_from_data(dt: &Data) -> Option<DType> {
        match dt {
            Data::Empty => None,
            Data::String(_) => Some(DType::String),
            Data::Float(_) => Some(DType::Numeric),
            Data::Int(_) => Some(DType::Integer),
            Data::Bool(_) => Some(DType::Boolean),
            Data::DateTime(_) | Data::DateTimeIso(_) => Some(DType::Date),
            Data::DurationIso(_) => Some(DType::Numeric),
            Data::Error(_) => None,
        }
    }

    fn process_sheet(
        &self,
        workbook: &mut Sheets<std::io::BufReader<std::fs::File>>,
        sheet_name: &str,
        sheet_idx: usize,
        options: &ProcessingOptions,
    ) -> Result<SheetSchema> {
        let range = workbook
            .worksheet_range(sheet_name)
            .map_err(crate::error::Error::Excel)?;

        let mut sheet = SheetSchema::new(sheet_name.to_string(), sheet_idx);

        if range.is_empty() {
            sheet.row_count = SafeValue::Integer(0);
            return Ok(sheet);
        }

        let (row_count, col_count) = range.get_size();

        if row_count == 0 {
            sheet.row_count = SafeValue::Integer(0);
            return Ok(sheet);
        }

        // First row is headers
        let headers: Vec<String> = range
            .rows()
            .next()
            .map(|row| {
                row.iter()
                    .map(|cell| Self::data_to_string(cell))
                    .collect()
            })
            .unwrap_or_default();

        let num_cols = headers.len().max(col_count);
        let data_rows = row_count.saturating_sub(1);

        // Initialize trackers
        let mut type_inferencers: Vec<TypeInferencer> =
            (0..num_cols).map(|_| TypeInferencer::new()).collect();
        let mut stat_trackers: Vec<ColumnStatTracker> = (0..num_cols)
            .map(|_| ColumnStatTracker::new(MAX_UNIQUE_VALUES))
            .collect();

        // Process data rows
        for row in range.rows().skip(1) {
            for (col_idx, cell) in row.iter().enumerate() {
                if col_idx >= num_cols {
                    continue;
                }

                // Type inference from Excel native type
                if Self::infer_type_from_data(cell).is_some() {
                    // Also use string inference for consistency
                    let str_val = Self::data_to_string(cell);
                    type_inferencers[col_idx].observe(&str_val);
                }

                // Statistics collection
                if Self::is_missing_data(cell) {
                    stat_trackers[col_idx].update_missing();
                } else if let Some(num) = Self::get_numeric_value(cell) {
                    let str_val = Self::data_to_string(cell);
                    stat_trackers[col_idx].update_numeric(num, &str_val);
                } else {
                    let str_val = Self::data_to_string(cell);
                    stat_trackers[col_idx].update_string(&str_val);
                }
            }
        }

        // Finalize type inference
        for inf in &mut type_inferencers {
            inf.finalize_initial_inference();
        }

        // Build column schemas
        let mut columns: Vec<ColumnSchema> = Vec::with_capacity(num_cols);

        for col_idx in 0..num_cols {
            let header = headers.get(col_idx).cloned().unwrap_or_else(|| format!("Column{}", col_idx + 1));
            let name_check = check_column_name(&header);
            let dtype = type_inferencers[col_idx].inferred_type();
            let tracker = &stat_trackers[col_idx];

            // Determine classification
            let mut classification = name_check.classification.clone();
            if tracker.unique_tracker.is_high_cardinality() {
                classification = Classification::HighCardinality;
            }

            // Build column name SafeValue
            let name_value = if classification == Classification::Phi {
                SafeValue::Suppressed {
                    reason: "Column name matches PHI pattern".to_string(),
                }
            } else {
                SafeValue::from_string(&header, "Column name too long")
            };

            let mut col_schema = ColumnSchema::new(name_value, col_idx, dtype);
            col_schema.classification = classification.clone();

            // Add warnings
            if let Some(warning) = name_check.warning {
                col_schema.warnings.push(warning);
            }

            // Build stats
            let mut stats = ColumnStats::default();
            let non_missing_count = tracker.count();
            stats.count = Some(safe_count(non_missing_count, options.bucket_counts));
            stats.missing_count = Some(safe_count(tracker.missing_count, options.bucket_counts));

            match dtype {
                DType::Integer | DType::Numeric => {
                    if let Some(min) = tracker.welford.min() {
                        stats.min = Some(SafeValue::Float(min));
                    }
                    if let Some(max) = tracker.welford.max() {
                        stats.max = Some(SafeValue::Float(max));
                    }
                    stats.mean = tracker.welford.mean();
                    stats.std_dev = tracker.welford.std_dev();
                    stats.median = tracker.p2_median.quantile();
                }
                _ => {}
            }

            // Unique count
            let unique_count = tracker.unique_tracker.unique_count() as u64;
            if tracker.unique_tracker.is_high_cardinality() {
                stats.unique_count = Some(SafeValue::Suppressed {
                    reason: "High cardinality; exact count suppressed".to_string(),
                });
            } else if options.bucket_counts {
                stats.unique_count =
                    Some(SafeValue::ShortString(bucket_count(unique_count).to_string()));
            } else {
                stats.unique_count = Some(SafeValue::Integer(unique_count as i64));
            }

            col_schema.stats = Some(stats);

            // Build unique values list (if safe)
            if classification == Classification::Safe || classification == Classification::Warning {
                if let Some(values) = tracker.unique_tracker.values() {
                    let mut safe_values: Vec<SafeValue> = Vec::new();
                    let counts = tracker.unique_tracker.value_counts();

                    for value in values {
                        let count = counts
                            .and_then(|c| c.get(value))
                            .copied()
                            .unwrap_or(1);

                        if count >= options.k_anonymity {
                            let value_check = crate::privacy::check_value_pattern(value);
                            if !value_check.is_phi && value.len() <= 32 {
                                safe_values.push(SafeValue::ShortString(value.clone()));
                            }
                        }
                    }

                    if !safe_values.is_empty() {
                        col_schema.unique_values = Some(safe_values);
                    }
                }
            }

            columns.push(col_schema);
        }

        sheet.row_count = safe_count(data_rows as u64, options.bucket_counts);
        sheet.columns = columns;

        Ok(sheet)
    }
}

impl DataReader for ExcelReader {
    fn read(&mut self, options: &ProcessingOptions) -> Result<Vec<SheetSchema>> {
        let mut workbook: Sheets<std::io::BufReader<std::fs::File>> =
            open_workbook_auto(&self.path)?;

        let sheet_names: Vec<String> = workbook.sheet_names().to_vec();
        let mut sheets: Vec<SheetSchema> = Vec::with_capacity(sheet_names.len());

        for (idx, sheet_name) in sheet_names.iter().enumerate() {
            let sheet = self.process_sheet(&mut workbook, sheet_name, idx, options)?;
            sheets.push(sheet);
        }

        Ok(sheets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_to_string() {
        assert_eq!(ExcelReader::data_to_string(&Data::Empty), "");
        assert_eq!(
            ExcelReader::data_to_string(&Data::String("test".to_string())),
            "test"
        );
        assert_eq!(
            ExcelReader::data_to_string(&Data::Int(42)),
            "42"
        );
        assert_eq!(
            ExcelReader::data_to_string(&Data::Float(3.14)),
            "3.14"
        );
        assert_eq!(
            ExcelReader::data_to_string(&Data::Bool(true)),
            "true"
        );
    }

    #[test]
    fn test_is_missing_data() {
        assert!(ExcelReader::is_missing_data(&Data::Empty));
        assert!(ExcelReader::is_missing_data(&Data::String("NA".to_string())));
        assert!(!ExcelReader::is_missing_data(&Data::Int(42)));
    }

    #[test]
    fn test_excel_serial_to_date() {
        // Excel serial date 44927 should be 2023-01-01
        let result = ExcelReader::excel_serial_to_date_string(44927.0);
        assert_eq!(result, "2023-01-01");
    }
}
