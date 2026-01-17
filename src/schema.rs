use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::readers::create_reader;
use crate::types::{FileFormat, ManifestSchema, ProcessingOptions, Result};

/// Result of schema extraction, including optional recode sidekick content
pub struct ExtractionResult {
    pub manifest: ManifestSchema,
    pub recode_sidekick: Option<String>,
}

/// Extract schema from a data file
pub fn extract_schema(path: &Path, options: ProcessingOptions) -> Result<ExtractionResult> {
    // Determine file format
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let format = FileFormat::from_extension(ext).ok_or_else(|| {
        crate::error::Error::UnsupportedFormat(format!(
            "Unsupported file extension: .{}",
            ext
        ))
    })?;

    // Get file name
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Create manifest
    let mut manifest = ManifestSchema::new(file_name, format);
    manifest.options = options.clone();

    // Compute file hash if requested
    if options.hash_file {
        manifest.file_hash = Some(compute_file_hash(path)?);
    }

    // Create reader and extract sheets with recoding
    let mut reader = create_reader(path)?;
    let (sheets, recode_registry) = reader.read_with_recoding(&options)?;
    manifest.sheets = sheets;

    // Generate recode sidekick content if any recoding was done
    let recode_sidekick = if recode_registry.has_recodings() {
        Some(recode_registry.generate_sidekick_content())
    } else {
        None
    };

    // Collect global warnings
    for sheet in &manifest.sheets {
        for col in &sheet.columns {
            if !col.warnings.is_empty() {
                for warning in &col.warnings {
                    let global_warning = format!(
                        "Sheet '{}', Column {}: {}",
                        sheet.name,
                        col.index + 1,
                        warning
                    );
                    if !manifest.warnings.contains(&global_warning) {
                        manifest.warnings.push(global_warning);
                    }
                }
            }
        }
    }

    Ok(ExtractionResult {
        manifest,
        recode_sidekick,
    })
}

/// Compute SHA-256 hash of a file (streaming to handle large files)
fn compute_file_hash(path: &Path) -> Result<String> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let result = hasher.finalize();
    Ok(format!("{:x}", result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_compute_file_hash() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, "test content").unwrap();

        let hash = compute_file_hash(file.path()).unwrap();
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // SHA-256 produces 64 hex chars
    }

    #[test]
    fn test_extract_schema_csv() {
        let mut file = NamedTempFile::with_suffix(".csv").unwrap();
        write!(file, "col1,col2\n1,a\n2,b\n3,c\n").unwrap();

        let options = ProcessingOptions::default();
        let result = extract_schema(file.path(), options).unwrap();

        assert_eq!(result.manifest.format, FileFormat::Csv);
        assert_eq!(result.manifest.sheets.len(), 1);
        assert_eq!(result.manifest.sheets[0].columns.len(), 2);
        assert!(result.manifest.file_hash.is_some());
    }

    #[test]
    fn test_extract_schema_unsupported() {
        let file = NamedTempFile::with_suffix(".xyz").unwrap();

        let options = ProcessingOptions::default();
        let result = extract_schema(file.path(), options);

        assert!(result.is_err());
    }

    #[test]
    fn test_extract_schema_with_recoding() {
        let mut file = NamedTempFile::with_suffix(".csv").unwrap();
        write!(file, "site_code,age\nVAN-001,30\nCAL-002,25\nVAN-001,35\n").unwrap();

        let options = ProcessingOptions::default();
        let result = extract_schema(file.path(), options).unwrap();

        // Check that recode sidekick was generated
        assert!(result.recode_sidekick.is_some());
        let sidekick = result.recode_sidekick.unwrap();
        assert!(sidekick.contains("Site_A"));
        assert!(sidekick.contains("Site_B"));
    }
}
