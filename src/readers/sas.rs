//! SAS .sas7bdat file reader (feature-gated)
//!
//! This module requires the `formats-readstat` feature to be enabled.

use std::path::{Path, PathBuf};

use crate::types::{ProcessingOptions, Result, SheetSchema};

use super::DataReader;

/// SAS .sas7bdat file reader
pub struct SasReader {
    path: PathBuf,
}

impl SasReader {
    pub fn new(path: &Path) -> Result<Self> {
        Ok(Self {
            path: path.to_path_buf(),
        })
    }
}

impl DataReader for SasReader {
    fn read(&mut self, _options: &ProcessingOptions) -> Result<Vec<SheetSchema>> {
        // TODO: Implement using readstat crate when available
        // For now, return an error indicating the feature is not fully implemented
        Err(crate::error::Error::ReadStat(
            "SAS reader not yet implemented. The readstat crate integration is pending.".to_string(),
        ))
    }
}
