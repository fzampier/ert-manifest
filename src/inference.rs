use chrono::NaiveDate;
use once_cell::sync::Lazy;
use regex::Regex;

use crate::types::{DType, TYPE_INFERENCE_SAMPLE_SIZE};

/// Boolean tokens (case-insensitive)
const TRUE_TOKENS: &[&str] = &["true", "yes", "y", "1", "t"];
const FALSE_TOKENS: &[&str] = &["false", "no", "n", "0", "f"];

/// Missing value tokens
pub const MISSING_TOKENS: &[&str] = &[
    "", "NA", "N/A", "na", "n/a", "NULL", "null", "NaN", "nan", ".", "-", "--", "missing",
    "MISSING", "None", "none", "#N/A", "#VALUE!", "#REF!", "#DIV/0!", "#NUM!", "#NAME?", "#NULL!",
];

// Date format patterns
static DATE_PATTERNS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    vec![
        // ISO format: 2024-01-15
        (Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap(), "%Y-%m-%d"),
        // US format: 01/15/2024 or 1/15/2024
        (
            Regex::new(r"^\d{1,2}/\d{1,2}/\d{4}$").unwrap(),
            "%m/%d/%Y",
        ),
        // European format: 15/01/2024 or 15-01-2024
        (
            Regex::new(r"^\d{1,2}-\d{1,2}-\d{4}$").unwrap(),
            "%d-%m-%Y",
        ),
        // Short year: 01/15/24
        (Regex::new(r"^\d{1,2}/\d{1,2}/\d{2}$").unwrap(), "%m/%d/%y"),
        // Month name: Jan 15, 2024 or January 15, 2024
        (
            Regex::new(r"^[A-Za-z]{3,9}\s+\d{1,2},?\s+\d{4}$").unwrap(),
            "%B %d, %Y",
        ),
        // ISO with dots: 2024.01.15
        (Regex::new(r"^\d{4}\.\d{2}\.\d{2}$").unwrap(), "%Y.%m.%d"),
    ]
});

// Datetime patterns
static DATETIME_PATTERNS: Lazy<Vec<(Regex, &'static str)>> = Lazy::new(|| {
    vec![
        // ISO datetime: 2024-01-15T10:30:00 or 2024-01-15 10:30:00
        (
            Regex::new(r"^\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}$").unwrap(),
            "%Y-%m-%dT%H:%M:%S",
        ),
        // With timezone: 2024-01-15T10:30:00Z
        (
            Regex::new(r"^\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}Z$").unwrap(),
            "%Y-%m-%dT%H:%M:%SZ",
        ),
        // With milliseconds: 2024-01-15T10:30:00.123
        (
            Regex::new(r"^\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}\.\d+$").unwrap(),
            "%Y-%m-%dT%H:%M:%S%.f",
        ),
    ]
});

/// Type inference state for a column
#[derive(Debug, Clone)]
pub struct TypeInferencer {
    /// Current inferred type
    current_type: Option<DType>,
    /// Sample values for initial inference
    samples: Vec<String>,
    /// Maximum sample size
    max_samples: usize,
    /// Number of values seen
    values_seen: u64,
    /// Whether initial inference is complete
    initial_inference_done: bool,
    /// Count of detected free text (long strings)
    free_text_count: u64,
}

impl TypeInferencer {
    pub fn new() -> Self {
        Self {
            current_type: None,
            samples: Vec::with_capacity(TYPE_INFERENCE_SAMPLE_SIZE),
            max_samples: TYPE_INFERENCE_SAMPLE_SIZE,
            values_seen: 0,
            initial_inference_done: false,
            free_text_count: 0,
        }
    }

    /// Add a value for type inference
    pub fn observe(&mut self, value: &str) {
        // Skip missing values
        if is_missing(value) {
            return;
        }

        self.values_seen += 1;

        if !self.initial_inference_done {
            // Collect samples
            if self.samples.len() < self.max_samples {
                self.samples.push(value.to_string());
            }

            // Do initial inference when we have enough samples or when called explicitly
            if self.samples.len() >= self.max_samples {
                self.perform_initial_inference();
            }
        } else {
            // Upgrade type if needed during full scan
            self.upgrade_type_if_needed(value);
        }
    }

    /// Force initial inference with current samples
    pub fn finalize_initial_inference(&mut self) {
        if !self.initial_inference_done && !self.samples.is_empty() {
            self.perform_initial_inference();
        }
    }

    /// Get the current inferred type
    pub fn inferred_type(&self) -> DType {
        self.current_type.unwrap_or(DType::String)
    }

    /// Perform initial type inference on collected samples
    fn perform_initial_inference(&mut self) {
        if self.samples.is_empty() {
            self.current_type = Some(DType::String);
            self.initial_inference_done = true;
            return;
        }

        // Try each type in order of specificity
        let dtype = if self.all_boolean(&self.samples) {
            DType::Boolean
        } else if self.all_integer(&self.samples) {
            DType::Integer
        } else if self.all_numeric(&self.samples) {
            DType::Numeric
        } else if self.all_datetime(&self.samples) {
            DType::Datetime
        } else if self.all_date(&self.samples) {
            DType::Date
        } else {
            DType::String
        };

        self.current_type = Some(dtype);
        self.initial_inference_done = true;

        // Clear samples to free memory
        self.samples.clear();
        self.samples.shrink_to_fit();
    }

    /// Upgrade type during full scan if value doesn't fit current type
    fn upgrade_type_if_needed(&mut self, value: &str) {
        let current = self.current_type.unwrap_or(DType::String);

        // Check for free text (long strings)
        if value.len() > 100 || value.contains('\n') {
            self.free_text_count += 1;
            if self.free_text_count > 10 && current == DType::String {
                self.current_type = Some(DType::FreeText);
                return;
            }
        }

        let new_type = match current {
            DType::Integer => {
                if !is_integer(value) {
                    if is_numeric(value) {
                        DType::Numeric
                    } else {
                        DType::String
                    }
                } else {
                    return;
                }
            }
            DType::Numeric => {
                if !is_numeric(value) {
                    DType::String
                } else {
                    return;
                }
            }
            DType::Boolean => {
                if !is_boolean(value) {
                    DType::String
                } else {
                    return;
                }
            }
            DType::Date => {
                if is_datetime(value) {
                    DType::Datetime
                } else if !is_date(value) {
                    DType::String
                } else {
                    return;
                }
            }
            DType::Datetime => {
                if !is_datetime(value) && !is_date(value) {
                    DType::String
                } else {
                    return;
                }
            }
            DType::String | DType::FreeText => {
                return; // Already most general
            }
        };

        self.current_type = Some(new_type);
    }

    fn all_boolean(&self, values: &[String]) -> bool {
        values.iter().all(|v| is_boolean(v))
    }

    fn all_integer(&self, values: &[String]) -> bool {
        values.iter().all(|v| is_integer(v))
    }

    fn all_numeric(&self, values: &[String]) -> bool {
        values.iter().all(|v| is_numeric(v))
    }

    fn all_date(&self, values: &[String]) -> bool {
        values.iter().all(|v| is_date(v))
    }

    fn all_datetime(&self, values: &[String]) -> bool {
        values.iter().all(|v| is_datetime(v))
    }
}

impl Default for TypeInferencer {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if a value represents a missing value
pub fn is_missing(value: &str) -> bool {
    let trimmed = value.trim();
    MISSING_TOKENS.iter().any(|t| trimmed.eq_ignore_ascii_case(t))
}

/// Check if a value is a boolean
pub fn is_boolean(value: &str) -> bool {
    let lower = value.trim().to_lowercase();
    TRUE_TOKENS.contains(&lower.as_str()) || FALSE_TOKENS.contains(&lower.as_str())
}

/// Check if a value is an integer
pub fn is_integer(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed.parse::<i64>().is_ok()
}

/// Check if a value is numeric (integer or float)
pub fn is_numeric(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    trimmed.parse::<f64>().is_ok()
}

/// Check if a value is a date
pub fn is_date(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }

    for (pattern, format) in DATE_PATTERNS.iter() {
        if pattern.is_match(trimmed) {
            if NaiveDate::parse_from_str(trimmed, format).is_ok() {
                return true;
            }
        }
    }
    false
}

/// Check if a value is a datetime
pub fn is_datetime(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }

    for (pattern, _) in DATETIME_PATTERNS.iter() {
        if pattern.is_match(trimmed) {
            return true;
        }
    }
    false
}

/// Parse a numeric value
pub fn parse_numeric(value: &str) -> Option<f64> {
    value.trim().parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_missing() {
        assert!(is_missing(""));
        assert!(is_missing("NA"));
        assert!(is_missing("N/A"));
        assert!(is_missing("null"));
        assert!(is_missing("NULL"));
        assert!(is_missing("."));
        assert!(is_missing("#N/A"));
        assert!(!is_missing("0"));
        assert!(!is_missing("test"));
    }

    #[test]
    fn test_is_boolean() {
        assert!(is_boolean("true"));
        assert!(is_boolean("True"));
        assert!(is_boolean("TRUE"));
        assert!(is_boolean("false"));
        assert!(is_boolean("yes"));
        assert!(is_boolean("no"));
        assert!(is_boolean("Y"));
        assert!(is_boolean("N"));
        assert!(is_boolean("1"));
        assert!(is_boolean("0"));
        assert!(!is_boolean("maybe"));
        assert!(!is_boolean("2"));
    }

    #[test]
    fn test_is_integer() {
        assert!(is_integer("42"));
        assert!(is_integer("-42"));
        assert!(is_integer("0"));
        assert!(!is_integer("3.14"));
        assert!(!is_integer("abc"));
        assert!(!is_integer(""));
    }

    #[test]
    fn test_is_numeric() {
        assert!(is_numeric("42"));
        assert!(is_numeric("3.14"));
        assert!(is_numeric("-3.14"));
        assert!(is_numeric("1e10"));
        assert!(!is_numeric("abc"));
        assert!(!is_numeric(""));
    }

    #[test]
    fn test_is_date() {
        assert!(is_date("2024-01-15"));
        assert!(is_date("01/15/2024"));
        assert!(!is_date("not a date"));
        assert!(!is_date(""));
    }

    #[test]
    fn test_is_datetime() {
        assert!(is_datetime("2024-01-15T10:30:00"));
        assert!(is_datetime("2024-01-15 10:30:00"));
        assert!(is_datetime("2024-01-15T10:30:00Z"));
        assert!(!is_datetime("2024-01-15")); // Date only
        assert!(!is_datetime("not a datetime"));
    }

    #[test]
    fn test_type_inferencer_integer() {
        let mut inf = TypeInferencer::new();
        inf.observe("1");
        inf.observe("2");
        inf.observe("3");
        inf.finalize_initial_inference();

        assert_eq!(inf.inferred_type(), DType::Integer);
    }

    #[test]
    fn test_type_inferencer_numeric() {
        let mut inf = TypeInferencer::new();
        inf.observe("1.5");
        inf.observe("2.5");
        inf.observe("3.5");
        inf.finalize_initial_inference();

        assert_eq!(inf.inferred_type(), DType::Numeric);
    }

    #[test]
    fn test_type_inferencer_boolean() {
        let mut inf = TypeInferencer::new();
        inf.observe("true");
        inf.observe("false");
        inf.observe("yes");
        inf.finalize_initial_inference();

        assert_eq!(inf.inferred_type(), DType::Boolean);
    }

    #[test]
    fn test_type_inferencer_date() {
        let mut inf = TypeInferencer::new();
        inf.observe("2024-01-15");
        inf.observe("2024-02-20");
        inf.observe("2024-03-25");
        inf.finalize_initial_inference();

        assert_eq!(inf.inferred_type(), DType::Date);
    }

    #[test]
    fn test_type_inferencer_upgrade_integer_to_numeric() {
        let mut inf = TypeInferencer::new();
        inf.observe("1");
        inf.observe("2");
        inf.finalize_initial_inference();

        assert_eq!(inf.inferred_type(), DType::Integer);

        // Now observe a float
        inf.observe("3.5");
        assert_eq!(inf.inferred_type(), DType::Numeric);
    }

    #[test]
    fn test_type_inferencer_upgrade_to_string() {
        let mut inf = TypeInferencer::new();
        inf.observe("1");
        inf.observe("2");
        inf.finalize_initial_inference();

        // Now observe a non-numeric
        inf.observe("abc");
        assert_eq!(inf.inferred_type(), DType::String);
    }

    #[test]
    fn test_type_inferencer_skips_missing() {
        let mut inf = TypeInferencer::new();
        inf.observe("1");
        inf.observe("NA");
        inf.observe("2");
        inf.observe("");
        inf.observe("3");
        inf.finalize_initial_inference();

        assert_eq!(inf.inferred_type(), DType::Integer);
    }
}
