use std::collections::HashMap;

/// Recoder for anonymizing site-identifying values
#[derive(Debug, Clone, Default)]
pub struct ValueRecoder {
    /// Maps original values to recoded values
    mappings: HashMap<String, String>,
    /// Counter for generating labels
    counter: usize,
    /// Prefix for recoded values (e.g., "Site" -> "Site_A", "Site_B")
    prefix: String,
}

impl ValueRecoder {
    /// Create a new recoder with the given prefix
    pub fn new(prefix: &str) -> Self {
        Self {
            mappings: HashMap::new(),
            counter: 0,
            prefix: prefix.to_string(),
        }
    }

    /// Create a recoder for site values
    #[allow(dead_code)]
    pub fn for_sites() -> Self {
        Self::new("Site")
    }

    /// Recode a value, returning the anonymized version
    /// Same input always produces same output (deterministic)
    pub fn recode(&mut self, original: &str) -> String {
        if let Some(recoded) = self.mappings.get(original) {
            return recoded.clone();
        }

        let label = self.generate_label();
        self.mappings.insert(original.to_string(), label.clone());
        label
    }

    /// Generate the next label (A, B, C, ... Z, AA, AB, ...)
    fn generate_label(&mut self) -> String {
        let label = index_to_label(self.counter);
        self.counter += 1;
        format!("{}_{}", self.prefix, label)
    }

    /// Get all mappings (original -> recoded)
    pub fn get_mappings(&self) -> &HashMap<String, String> {
        &self.mappings
    }

    /// Get the reverse mapping (recoded -> original) for the sidekick file
    pub fn get_reverse_mappings(&self) -> HashMap<String, String> {
        self.mappings
            .iter()
            .map(|(k, v)| (v.clone(), k.clone()))
            .collect()
    }

    /// Get mapping count
    pub fn count(&self) -> usize {
        self.mappings.len()
    }
}

/// Convert a 0-based index to a letter label (0=A, 1=B, ..., 25=Z, 26=AA, ...)
fn index_to_label(index: usize) -> String {
    let mut result = String::new();
    let mut n = index;

    loop {
        let remainder = n % 26;
        result.insert(0, (b'A' + remainder as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }

    result
}

/// Collection of recoders for multiple columns
#[derive(Debug, Clone, Default)]
pub struct RecodeRegistry {
    /// Maps column index to its recoder
    recoders: HashMap<usize, ValueRecoder>,
    /// Maps column index to column name (for sidekick file)
    column_names: HashMap<usize, String>,
}

impl RecodeRegistry {
    pub fn new() -> Self {
        Self {
            recoders: HashMap::new(),
            column_names: HashMap::new(),
        }
    }

    /// Register a column for recoding
    pub fn register_column(&mut self, column_index: usize, column_name: &str, prefix: &str) {
        self.recoders.insert(column_index, ValueRecoder::new(prefix));
        self.column_names.insert(column_index, column_name.to_string());
    }

    /// Recode a value for a specific column
    pub fn recode(&mut self, column_index: usize, original: &str) -> Option<String> {
        self.recoders.get_mut(&column_index).map(|r| r.recode(original))
    }

    /// Check if a column is registered for recoding
    pub fn is_recoded(&self, column_index: usize) -> bool {
        self.recoders.contains_key(&column_index)
    }

    /// Get all recoded values for a column
    pub fn get_recoded_values(&self, column_index: usize) -> Option<Vec<String>> {
        self.recoders.get(&column_index).map(|r| {
            let mut values: Vec<_> = r.get_mappings().values().cloned().collect();
            values.sort();
            values
        })
    }

    /// Generate the sidekick file content
    pub fn generate_sidekick_content(&self) -> String {
        let mut lines = Vec::new();
        lines.push("# ERT-Manifest Recode Mapping".to_string());
        lines.push("# CONFIDENTIAL - Keep this file secure at your site".to_string());
        lines.push(format!("# Generated: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        lines.push(String::new());

        // Sort by column index for consistent output
        let mut indices: Vec<_> = self.recoders.keys().collect();
        indices.sort();

        for &col_idx in &indices {
            if let (Some(recoder), Some(col_name)) = (self.recoders.get(col_idx), self.column_names.get(col_idx)) {
                lines.push(format!("## Column {}: {}", col_idx + 1, col_name));
                lines.push(String::new());

                // Sort mappings by recoded value for readability
                let reverse = recoder.get_reverse_mappings();
                let mut recoded_values: Vec<_> = reverse.keys().collect();
                recoded_values.sort();

                for recoded in recoded_values {
                    if let Some(original) = reverse.get(recoded) {
                        lines.push(format!("{} = {}", recoded, original));
                    }
                }
                lines.push(String::new());
            }
        }

        lines.join("\n")
    }

    /// Check if any recoding was done
    pub fn has_recodings(&self) -> bool {
        self.recoders.values().any(|r| r.count() > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_index_to_label() {
        assert_eq!(index_to_label(0), "A");
        assert_eq!(index_to_label(1), "B");
        assert_eq!(index_to_label(25), "Z");
        assert_eq!(index_to_label(26), "AA");
        assert_eq!(index_to_label(27), "AB");
        assert_eq!(index_to_label(51), "AZ");
        assert_eq!(index_to_label(52), "BA");
    }

    #[test]
    fn test_value_recoder() {
        let mut recoder = ValueRecoder::for_sites();

        assert_eq!(recoder.recode("YVR-003"), "Site_A");
        assert_eq!(recoder.recode("YYC-001"), "Site_B");
        assert_eq!(recoder.recode("YVR-003"), "Site_A"); // Same input, same output
        assert_eq!(recoder.recode("YEG-002"), "Site_C");

        assert_eq!(recoder.count(), 3);
    }

    #[test]
    fn test_recode_registry() {
        let mut registry = RecodeRegistry::new();
        registry.register_column(5, "site_code", "Site");

        assert!(registry.is_recoded(5));
        assert!(!registry.is_recoded(0));

        assert_eq!(registry.recode(5, "YVR-003"), Some("Site_A".to_string()));
        assert_eq!(registry.recode(5, "YYC-001"), Some("Site_B".to_string()));
        assert_eq!(registry.recode(0, "test"), None);
    }

    #[test]
    fn test_sidekick_content() {
        let mut registry = RecodeRegistry::new();
        registry.register_column(5, "site_code", "Site");
        registry.recode(5, "Vancouver General");
        registry.recode(5, "Calgary Foothills");

        let content = registry.generate_sidekick_content();
        assert!(content.contains("Column 6: site_code"));
        assert!(content.contains("Site_A = "));
        assert!(content.contains("Site_B = "));
    }
}
