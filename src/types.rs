use serde::{Deserialize, Serialize};

/// Maximum length for short strings that can be safely exported
pub const MAX_SHORT_STRING_LEN: usize = 32;

/// Maximum unique values to track before marking as high cardinality
pub const MAX_UNIQUE_VALUES: usize = 2000;

/// Default k-anonymity threshold
pub const DEFAULT_K_ANONYMITY: u64 = 5;

/// Sample size for type inference
pub const TYPE_INFERENCE_SAMPLE_SIZE: usize = 2000;

/// A value that is safe to export (privacy-preserving)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum SafeValue {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    ShortString(String),
    Suppressed { reason: String },
}

impl SafeValue {
    /// Create a SafeValue from a string, enforcing length limits
    pub fn from_string(s: &str, reason_if_too_long: &str) -> Self {
        if s.len() > MAX_SHORT_STRING_LEN {
            SafeValue::Suppressed {
                reason: reason_if_too_long.to_string(),
            }
        } else {
            SafeValue::ShortString(s.to_string())
        }
    }
}

/// Data type classification for columns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DType {
    Integer,
    Numeric,
    String,
    Date,
    Datetime,
    Boolean,
    FreeText,
}


/// Classification of a column's privacy sensitivity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Classification {
    /// Safe to export
    Safe,
    /// Potentially sensitive, warn but allow
    Warning,
    /// Contains PHI, suppress values
    Phi,
    /// Contains site-identifying info, recode to anonymous labels
    Recode,
    /// High cardinality, suppress unique values
    HighCardinality,
}

/// Statistics for a column (all privacy-safe)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ColumnStats {
    /// Count of non-missing values (may be bucketed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<SafeValue>,

    /// Count of missing values (may be bucketed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub missing_count: Option<SafeValue>,

    /// Minimum value (for numeric/date types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<SafeValue>,

    /// Maximum value (for numeric/date types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<SafeValue>,

    /// Mean value (for numeric types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mean: Option<f64>,

    /// Standard deviation (for numeric types)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub std_dev: Option<f64>,

    /// Median value (for numeric types, estimated via P²)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub median: Option<f64>,

    /// Number of unique values (may be bucketed or marked high cardinality)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_count: Option<SafeValue>,
}

/// Schema for a single column
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSchema {
    /// Column name (may be suppressed if PHI)
    pub name: SafeValue,

    /// Column index (0-based)
    pub index: usize,

    /// Inferred data type
    pub dtype: DType,

    /// Privacy classification
    pub classification: Classification,

    /// Column statistics
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<ColumnStats>,

    /// Unique values (only if safe to export)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unique_values: Option<Vec<SafeValue>>,

    /// Warnings about this column
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

impl ColumnSchema {
    pub fn new(name: SafeValue, index: usize, dtype: DType) -> Self {
        Self {
            name,
            index,
            dtype,
            classification: Classification::Safe,
            stats: None,
            unique_values: None,
            warnings: Vec::new(),
        }
    }
}

/// Schema for a single sheet (or table)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetSchema {
    /// Sheet name (for Excel) or file name (for CSV)
    pub name: String,

    /// Sheet index (0-based)
    pub index: usize,

    /// Row count (may be bucketed)
    pub row_count: SafeValue,

    /// Column schemas
    pub columns: Vec<ColumnSchema>,

    /// Sheet-level warnings
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

impl SheetSchema {
    pub fn new(name: String, index: usize) -> Self {
        Self {
            name,
            index,
            row_count: SafeValue::Integer(0),
            columns: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

/// Complete manifest schema for a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSchema {
    /// Schema version
    pub version: String,

    /// File name (without path)
    pub file_name: String,

    /// File hash (SHA-256)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_hash: Option<String>,

    /// File format
    pub format: FileFormat,

    /// Sheets in the file
    pub sheets: Vec<SheetSchema>,

    /// Global warnings
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,

    /// Processing options used
    pub options: ProcessingOptions,
}

impl ManifestSchema {
    pub fn new(file_name: String, format: FileFormat) -> Self {
        Self {
            version: "1.0.0".to_string(),
            file_name,
            file_hash: None,
            format,
            sheets: Vec::new(),
            warnings: Vec::new(),
            options: ProcessingOptions::default(),
        }
    }
}

/// Supported file formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileFormat {
    Csv,
    Tsv,
    Excel,
}

impl FileFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "csv" => Some(FileFormat::Csv),
            "tsv" | "tab" => Some(FileFormat::Tsv),
            "xlsx" | "xls" | "xlsm" | "xlsb" => Some(FileFormat::Excel),
            _ => None,
        }
    }
}

/// Processing options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingOptions {
    /// K-anonymity threshold
    pub k_anonymity: u64,

    /// Whether to bucket counts
    pub bucket_counts: bool,

    /// Whether to use exact counts (requires --relaxed)
    pub exact_counts: bool,

    /// Whether to use exact median (requires --relaxed)
    pub exact_median: bool,

    /// Whether to hash the file
    pub hash_file: bool,

    /// Relaxed mode (allows exact counts/median)
    pub relaxed: bool,
}

impl Default for ProcessingOptions {
    fn default() -> Self {
        Self {
            k_anonymity: DEFAULT_K_ANONYMITY,
            bucket_counts: true,
            exact_counts: false,
            exact_median: false,
            hash_file: true,
            relaxed: false,
        }
    }
}

/// Result type for the application
pub type Result<T> = std::result::Result<T, crate::error::Error>;
