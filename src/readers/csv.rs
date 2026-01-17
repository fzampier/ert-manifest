use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use csv::{Reader, ReaderBuilder};

use crate::inference::{is_missing, parse_numeric, TypeInferencer};
use crate::privacy::{bucket_count, check_column_name, safe_count, RecodeRegistry};
use crate::stats::ColumnStatTracker;
use crate::types::{
    Classification, ColumnSchema, ColumnStats, DType, ProcessingOptions, Result, SafeValue,
    SheetSchema, MAX_UNIQUE_VALUES,
};

use super::DataReader;

/// CSV/TSV file reader
pub struct CsvReader {
    path: PathBuf,
    delimiter: u8,
}

impl CsvReader {
    /// Create a new CSV reader
    pub fn new(path: &Path) -> Result<Self> {
        Ok(Self {
            path: path.to_path_buf(),
            delimiter: b',',
        })
    }

    /// Create a new TSV reader
    pub fn new_tsv(path: &Path) -> Result<Self> {
        Ok(Self {
            path: path.to_path_buf(),
            delimiter: b'\t',
        })
    }

    fn create_reader(&self) -> Result<Reader<BufReader<File>>> {
        let file = File::open(&self.path)?;
        let reader = BufReader::new(file);
        let csv_reader = ReaderBuilder::new()
            .delimiter(self.delimiter)
            .has_headers(true)
            .flexible(true)
            .from_reader(reader);
        Ok(csv_reader)
    }
}

impl DataReader for CsvReader {
    fn read(&mut self, options: &ProcessingOptions) -> Result<Vec<SheetSchema>> {
        let (sheets, _recode_registry) = self.read_with_recoding(options)?;
        Ok(sheets)
    }

    fn read_with_recoding(&mut self, options: &ProcessingOptions) -> Result<(Vec<SheetSchema>, RecodeRegistry)> {
        let mut reader = self.create_reader()?;

        // Get headers
        let headers: Vec<String> = reader
            .headers()?
            .iter()
            .map(|h| h.to_string())
            .collect();

        let num_cols = headers.len();

        // Check column names and set up recoding registry
        let mut recode_registry = RecodeRegistry::new();
        let column_checks: Vec<_> = headers.iter().map(|h| check_column_name(h)).collect();

        for (col_idx, check) in column_checks.iter().enumerate() {
            if check.classification == Classification::Recode {
                // Determine prefix based on column name
                let prefix = determine_recode_prefix(&headers[col_idx]);
                recode_registry.register_column(col_idx, &headers[col_idx], &prefix);
            }
        }

        // Initialize trackers for each column
        let mut type_inferencers: Vec<TypeInferencer> =
            (0..num_cols).map(|_| TypeInferencer::new()).collect();
        let mut stat_trackers: Vec<ColumnStatTracker> = (0..num_cols)
            .map(|_| ColumnStatTracker::new(MAX_UNIQUE_VALUES))
            .collect();

        // First pass: collect samples for type inference
        let mut row_count: u64 = 0;

        for result in reader.records() {
            let record = result?;
            row_count += 1;

            for (col_idx, field) in record.iter().enumerate() {
                if col_idx >= num_cols {
                    continue;
                }

                type_inferencers[col_idx].observe(field);
            }
        }

        // Finalize type inference
        for inf in &mut type_inferencers {
            inf.finalize_initial_inference();
        }

        // Second pass: collect statistics (with recoding)
        let mut reader = self.create_reader()?;
        for result in reader.records() {
            let record = result?;

            for (col_idx, field) in record.iter().enumerate() {
                if col_idx >= num_cols {
                    continue;
                }

                let dtype = type_inferencers[col_idx].inferred_type();

                if is_missing(field) {
                    stat_trackers[col_idx].update_missing();
                } else {
                    // Recode values if this column is marked for recoding
                    let value_to_track = if recode_registry.is_recoded(col_idx) {
                        recode_registry.recode(col_idx, field).unwrap_or_else(|| field.to_string())
                    } else {
                        field.to_string()
                    };

                    match dtype {
                        DType::Integer | DType::Numeric => {
                            if let Some(num) = parse_numeric(field) {
                                stat_trackers[col_idx].update_numeric(num, &value_to_track);
                            } else {
                                stat_trackers[col_idx].update_string(&value_to_track);
                            }
                        }
                        _ => {
                            stat_trackers[col_idx].update_string(&value_to_track);
                        }
                    }
                }
            }
        }

        // Build column schemas
        let mut columns: Vec<ColumnSchema> = Vec::with_capacity(num_cols);

        for (col_idx, header) in headers.iter().enumerate() {
            let name_check = &column_checks[col_idx];
            let dtype = type_inferencers[col_idx].inferred_type();
            let tracker = &stat_trackers[col_idx];

            // Determine classification
            let mut classification = name_check.classification.clone();
            if tracker.unique_tracker.is_high_cardinality()
                && classification != Classification::Recode
                && classification != Classification::Phi
            {
                classification = Classification::HighCardinality;
            }

            // Build column name SafeValue
            let name_value = if classification == Classification::Phi {
                SafeValue::Suppressed {
                    reason: "Column name matches PHI pattern".to_string(),
                }
            } else {
                SafeValue::from_string(header, "Column name too long")
            };

            let mut col_schema = ColumnSchema::new(name_value, col_idx, dtype);
            col_schema.classification = classification.clone();

            // Add warnings
            if let Some(warning) = &name_check.warning {
                col_schema.warnings.push(warning.clone());
            }

            // Build stats
            let mut stats = ColumnStats::default();
            let non_missing_count = tracker.welford.count();
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
            if tracker.unique_tracker.is_high_cardinality() && classification != Classification::Recode {
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

            // Build unique values list
            if classification == Classification::Recode {
                // For recoded columns, show the recoded values
                if let Some(recoded_values) = recode_registry.get_recoded_values(col_idx) {
                    let safe_values: Vec<SafeValue> = recoded_values
                        .into_iter()
                        .map(SafeValue::ShortString)
                        .collect();
                    if !safe_values.is_empty() {
                        col_schema.unique_values = Some(safe_values);
                    }
                }
            } else if classification == Classification::Safe || classification == Classification::Warning {
                if let Some(values) = tracker.unique_tracker.values() {
                    let mut safe_values: Vec<SafeValue> = Vec::new();
                    let counts = tracker.unique_tracker.value_counts();

                    for value in values {
                        let count = counts
                            .and_then(|c| c.get(value))
                            .copied()
                            .unwrap_or(1);

                        if count >= options.k_anonymity {
                            // Check value-level privacy
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

        // Build sheet schema
        let file_name = self
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let mut sheet = SheetSchema::new(file_name, 0);
        sheet.row_count = safe_count(row_count, options.bucket_counts);
        sheet.columns = columns;

        Ok((vec![sheet], recode_registry))
    }
}

/// Determine the appropriate prefix for recoding based on column name
fn determine_recode_prefix(column_name: &str) -> String {
    let lower = column_name.to_lowercase();
    if lower.contains("hospital") {
        "Hospital".to_string()
    } else if lower.contains("clinic") {
        "Clinic".to_string()
    } else if lower.contains("facility") {
        "Facility".to_string()
    } else if lower.contains("center") || lower.contains("centre") {
        "Center".to_string()
    } else if lower.contains("location") {
        "Location".to_string()
    } else {
        "Site".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_csv(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::with_suffix(".csv").unwrap();
        write!(file, "{}", content).unwrap();
        file
    }

    #[test]
    fn test_basic_csv_read() {
        let csv_content = "id,name,age\n1,Alice,30\n2,Bob,25\n3,Charlie,35\n";
        let file = create_test_csv(csv_content);

        let mut reader = CsvReader::new(file.path()).unwrap();
        let options = ProcessingOptions::default();
        let sheets = reader.read(&options).unwrap();

        assert_eq!(sheets.len(), 1);
        assert_eq!(sheets[0].columns.len(), 3);
    }

    #[test]
    fn test_type_inference() {
        let csv_content = "int_col,float_col,str_col\n1,1.5,hello\n2,2.5,world\n3,3.5,test\n";
        let file = create_test_csv(csv_content);

        let mut reader = CsvReader::new(file.path()).unwrap();
        let options = ProcessingOptions::default();
        let sheets = reader.read(&options).unwrap();

        assert_eq!(sheets[0].columns[0].dtype, DType::Integer);
        assert_eq!(sheets[0].columns[1].dtype, DType::Numeric);
        assert_eq!(sheets[0].columns[2].dtype, DType::String);
    }

    #[test]
    fn test_phi_column_detection() {
        let csv_content = "patient_name,age\nJohn Doe,30\nJane Smith,25\n";
        let file = create_test_csv(csv_content);

        let mut reader = CsvReader::new(file.path()).unwrap();
        let options = ProcessingOptions::default();
        let sheets = reader.read(&options).unwrap();

        assert_eq!(sheets[0].columns[0].classification, Classification::Phi);
        assert!(!sheets[0].columns[0].warnings.is_empty());
    }

    #[test]
    fn test_missing_values() {
        // CSV with explicit missing values (NA and empty string in a cell)
        let csv_content = "col,col2\n1,a\nNA,b\n2,c\n,d\n3,e\n";
        let file = create_test_csv(csv_content);

        let mut reader = CsvReader::new(file.path()).unwrap();
        let options = ProcessingOptions {
            bucket_counts: false,
            ..ProcessingOptions::default()
        };
        let sheets = reader.read(&options).unwrap();

        let stats = sheets[0].columns[0].stats.as_ref().unwrap();
        assert_eq!(stats.count, Some(SafeValue::Integer(3))); // 1, 2, 3
        assert_eq!(stats.missing_count, Some(SafeValue::Integer(2))); // NA and empty
    }
}
