use crate::types::{ManifestSchema, Result};
use std::io::Write;
use std::path::Path;

/// Write manifest to JSON file
pub fn write_json_file(manifest: &ManifestSchema, path: &Path) -> Result<()> {
    let file = std::fs::File::create(path)?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, manifest)?;
    Ok(())
}

/// Write manifest to JSON string
pub fn to_json_string(manifest: &ManifestSchema) -> Result<String> {
    Ok(serde_json::to_string_pretty(manifest)?)
}

/// Write manifest to stdout
pub fn write_json_stdout(manifest: &ManifestSchema) -> Result<()> {
    let json = to_json_string(manifest)?;
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    writeln!(handle, "{}", json)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{FileFormat, SafeValue, SheetSchema};

    #[test]
    fn test_json_serialization() {
        let mut manifest = ManifestSchema::new("test.csv".to_string(), FileFormat::Csv);
        let mut sheet = SheetSchema::new("Sheet1".to_string(), 0);
        sheet.row_count = SafeValue::Integer(100);
        manifest.sheets.push(sheet);

        let json = to_json_string(&manifest).unwrap();
        assert!(json.contains("\"file_name\": \"test.csv\""));
        assert!(json.contains("\"format\": \"csv\""));
    }
}
