use crate::types::{Classification, SafeValue, MAX_SHORT_STRING_LEN};

use super::value_patterns::check_value_pattern;

/// Reason for suppressing a value
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum SuppressionReason {
    /// Value count below k-anonymity threshold
    BelowKThreshold { count: u64, k: u64 },
    /// Column marked as PHI
    PhiColumn { pattern: String },
    /// Value matches PHI pattern
    PhiValue { pattern: String, description: String },
    /// Value too long
    TooLong { length: usize, max: usize },
    /// High cardinality column
    HighCardinality,
}

impl SuppressionReason {
    pub fn to_string(&self) -> String {
        match self {
            SuppressionReason::BelowKThreshold { count, k } => {
                format!("Count {} below k-anonymity threshold {}", count, k)
            }
            SuppressionReason::PhiColumn { pattern } => {
                format!("Column matches PHI pattern '{}'", pattern)
            }
            SuppressionReason::PhiValue { pattern, description } => {
                format!("{} (pattern: {})", description, pattern)
            }
            SuppressionReason::TooLong { length, max } => {
                format!("Value length {} exceeds maximum {}", length, max)
            }
            SuppressionReason::HighCardinality => {
                "High cardinality column; unique values suppressed".to_string()
            }
        }
    }
}

/// Check if a value should be suppressed and why
#[allow(dead_code)]
pub fn should_suppress_value(
    value: &str,
    count: u64,
    k: u64,
    column_classification: &Classification,
    phi_pattern: Option<&str>,
) -> Option<SuppressionReason> {
    // Check if column is marked as PHI
    if *column_classification == Classification::Phi {
        return Some(SuppressionReason::PhiColumn {
            pattern: phi_pattern.unwrap_or("unknown").to_string(),
        });
    }

    // Check value length
    if value.len() > MAX_SHORT_STRING_LEN {
        return Some(SuppressionReason::TooLong {
            length: value.len(),
            max: MAX_SHORT_STRING_LEN,
        });
    }

    // Check k-anonymity
    if count < k {
        return Some(SuppressionReason::BelowKThreshold { count, k });
    }

    // Check value patterns
    let pattern_result = check_value_pattern(value);
    if pattern_result.is_phi {
        return Some(SuppressionReason::PhiValue {
            pattern: pattern_result.matched_pattern.unwrap_or("unknown").to_string(),
            description: pattern_result
                .description
                .unwrap_or("PHI detected")
                .to_string(),
        });
    }

    None
}

/// Create a SafeValue from a string, applying suppression rules
#[allow(dead_code)]
pub fn safe_string_value(
    value: &str,
    count: u64,
    k: u64,
    column_classification: &Classification,
    phi_pattern: Option<&str>,
) -> SafeValue {
    if let Some(reason) = should_suppress_value(value, count, k, column_classification, phi_pattern)
    {
        SafeValue::Suppressed {
            reason: reason.to_string(),
        }
    } else {
        SafeValue::ShortString(value.to_string())
    }
}

/// Check if a value passes all safety checks (for inclusion in unique values list)
#[allow(dead_code)]
pub fn is_safe_for_export(
    value: &str,
    count: u64,
    k: u64,
    column_classification: &Classification,
) -> bool {
    should_suppress_value(value, count, k, column_classification, None).is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suppress_below_k() {
        let result =
            should_suppress_value("test", 3, 5, &Classification::Safe, None);
        assert!(matches!(
            result,
            Some(SuppressionReason::BelowKThreshold { count: 3, k: 5 })
        ));
    }

    #[test]
    fn test_no_suppress_at_k() {
        let result =
            should_suppress_value("test", 5, 5, &Classification::Safe, None);
        assert!(result.is_none());
    }

    #[test]
    fn test_suppress_phi_column() {
        let result = should_suppress_value(
            "John",
            100,
            5,
            &Classification::Phi,
            Some("name"),
        );
        assert!(matches!(result, Some(SuppressionReason::PhiColumn { .. })));
    }

    #[test]
    fn test_suppress_phi_value() {
        let result = should_suppress_value(
            "john@example.com",
            100,
            5,
            &Classification::Safe,
            None,
        );
        assert!(matches!(result, Some(SuppressionReason::PhiValue { .. })));
    }

    #[test]
    fn test_suppress_long_value() {
        let long_value = "a".repeat(50);
        let result =
            should_suppress_value(&long_value, 100, 5, &Classification::Safe, None);
        assert!(matches!(result, Some(SuppressionReason::TooLong { .. })));
    }

    #[test]
    fn test_safe_value() {
        let result =
            should_suppress_value("Treatment A", 100, 5, &Classification::Safe, None);
        assert!(result.is_none());
    }

    #[test]
    fn test_safe_string_value_suppressed() {
        let result = safe_string_value(
            "john@example.com",
            100,
            5,
            &Classification::Safe,
            None,
        );
        assert!(matches!(result, SafeValue::Suppressed { .. }));
    }

    #[test]
    fn test_safe_string_value_allowed() {
        let result = safe_string_value("Male", 100, 5, &Classification::Safe, None);
        assert_eq!(result, SafeValue::ShortString("Male".to_string()));
    }
}
