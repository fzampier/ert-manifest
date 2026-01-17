pub mod csv;
pub mod excel;

#[cfg(feature = "formats-readstat")]
pub mod sas;
#[cfg(feature = "formats-readstat")]
pub mod spss;
#[cfg(feature = "formats-readstat")]
pub mod stata;

use std::path::Path;

use crate::privacy::RecodeRegistry;
use crate::types::{FileFormat, ProcessingOptions, Result, SheetSchema};

/// Common trait for data file readers
pub trait DataReader {
    /// Read the file and return sheet schemas
    fn read(&mut self, options: &ProcessingOptions) -> Result<Vec<SheetSchema>>;

    /// Read the file with recoding support, returning both schemas and recode registry
    fn read_with_recoding(&mut self, options: &ProcessingOptions) -> Result<(Vec<SheetSchema>, RecodeRegistry)> {
        // Default implementation: no recoding
        let sheets = self.read(options)?;
        Ok((sheets, RecodeRegistry::new()))
    }
}

/// Create a reader for the given file path
pub fn create_reader(path: &Path) -> Result<Box<dyn DataReader>> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let format = FileFormat::from_extension(ext).ok_or_else(|| {
        crate::error::Error::UnsupportedFormat(format!(
            "Unknown file extension: {}",
            ext
        ))
    })?;

    match format {
        FileFormat::Csv => Ok(Box::new(csv::CsvReader::new(path)?)),
        FileFormat::Tsv => Ok(Box::new(csv::CsvReader::new_tsv(path)?)),
        FileFormat::Excel => Ok(Box::new(excel::ExcelReader::new(path)?)),
        #[cfg(feature = "formats-readstat")]
        FileFormat::Stata => Ok(Box::new(stata::StataReader::new(path)?)),
        #[cfg(feature = "formats-readstat")]
        FileFormat::Sas => Ok(Box::new(sas::SasReader::new(path)?)),
        #[cfg(feature = "formats-readstat")]
        FileFormat::Spss => Ok(Box::new(spss::SpssReader::new(path)?)),
    }
}
