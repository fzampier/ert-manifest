use once_cell::sync::Lazy;
use regex::Regex;

use super::name_lists::is_likely_name;

/// Result of checking a value for PHI patterns
#[derive(Debug, Clone, PartialEq)]
pub struct ValuePatternResult {
    pub is_phi: bool,
    pub matched_pattern: Option<&'static str>,
    pub description: Option<&'static str>,
}

impl ValuePatternResult {
    pub fn safe() -> Self {
        Self {
            is_phi: false,
            matched_pattern: None,
            description: None,
        }
    }

    pub fn phi(pattern: &'static str, description: &'static str) -> Self {
        Self {
            is_phi: true,
            matched_pattern: Some(pattern),
            description: Some(description),
        }
    }
}

// Compiled regex patterns for PHI detection
static EMAIL_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$").unwrap()
});

static US_PHONE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\(?\d{3}\)?[-.\s]?\d{3}[-.\s]?\d{4}$").unwrap()
});

static US_ZIP_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\d{5}(-\d{4})?$").unwrap());

static CANADA_POSTAL_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[A-Za-z]\d[A-Za-z]\s?\d[A-Za-z]\d$").unwrap());

static SSN_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\d{3}-?\d{2}-?\d{4}$").unwrap());

static LONG_ID_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[A-Za-z0-9]{10,}$").unwrap());

// HIPAA #14: URLs
static URL_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^https?://[^\s]+$").unwrap());

// HIPAA #15: IP addresses
static IPV4_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$").unwrap());

static IPV6_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}$|^([0-9a-fA-F]{1,4}:){1,7}:$|^::[0-9a-fA-F]{1,4}(:[0-9a-fA-F]{1,4}){0,6}$").unwrap());

// HIPAA #13: MAC addresses
static MAC_ADDRESS_PATTERN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([0-9A-Fa-f]{2}[:-]){5}[0-9A-Fa-f]{2}$").unwrap());

/// Check if a value matches any PHI pattern
pub fn check_value_pattern(value: &str) -> ValuePatternResult {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        return ValuePatternResult::safe();
    }

    // Check email pattern
    if EMAIL_PATTERN.is_match(trimmed) {
        return ValuePatternResult::phi("email", "Value appears to be an email address");
    }

    // Check US phone pattern
    if US_PHONE_PATTERN.is_match(trimmed) {
        return ValuePatternResult::phi("phone", "Value appears to be a phone number");
    }

    // Check SSN pattern
    if SSN_PATTERN.is_match(trimmed) {
        return ValuePatternResult::phi("ssn", "Value appears to be a Social Security Number");
    }

    // Check US ZIP code pattern
    if US_ZIP_PATTERN.is_match(trimmed) {
        return ValuePatternResult::phi("zip", "Value appears to be a US ZIP code");
    }

    // Check Canada postal code pattern
    if CANADA_POSTAL_PATTERN.is_match(trimmed) {
        return ValuePatternResult::phi("postal", "Value appears to be a Canadian postal code");
    }

    // Check for long alphanumeric IDs with mixed letters and digits
    if is_suspicious_long_id(trimmed) {
        return ValuePatternResult::phi(
            "long_id",
            "Value appears to be a long alphanumeric identifier",
        );
    }

    // Check URL pattern (HIPAA #14)
    if URL_PATTERN.is_match(trimmed) {
        return ValuePatternResult::phi("url", "Value appears to be a URL");
    }

    // Check IPv4 pattern (HIPAA #15)
    if IPV4_PATTERN.is_match(trimmed) {
        return ValuePatternResult::phi("ipv4", "Value appears to be an IPv4 address");
    }

    // Check IPv6 pattern (HIPAA #15)
    if IPV6_PATTERN.is_match(trimmed) {
        return ValuePatternResult::phi("ipv6", "Value appears to be an IPv6 address");
    }

    // Check MAC address pattern (HIPAA #13)
    if MAC_ADDRESS_PATTERN.is_match(trimmed) {
        return ValuePatternResult::phi("mac_address", "Value appears to be a MAC address");
    }

    // Check for person names (HIPAA #1)
    if is_likely_name(trimmed) {
        return ValuePatternResult::phi("name", "Value appears to be a person's name");
    }

    ValuePatternResult::safe()
}

/// Check if a value looks like a suspicious long alphanumeric ID
fn is_suspicious_long_id(value: &str) -> bool {
    if !LONG_ID_PATTERN.is_match(value) {
        return false;
    }

    // Must have both letters and digits to be suspicious
    let has_letters = value.chars().any(|c| c.is_ascii_alphabetic());
    let has_digits = value.chars().any(|c| c.is_ascii_digit());

    has_letters && has_digits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_detection() {
        assert!(check_value_pattern("john.doe@example.com").is_phi);
        assert!(check_value_pattern("test@test.org").is_phi);
        assert!(check_value_pattern("user123@company.co.uk").is_phi);
    }

    #[test]
    fn test_phone_detection() {
        assert!(check_value_pattern("555-123-4567").is_phi);
        assert!(check_value_pattern("5551234567").is_phi);
        assert!(check_value_pattern("(555) 123-4567").is_phi);
        assert!(check_value_pattern("555.123.4567").is_phi);
    }

    #[test]
    fn test_ssn_detection() {
        assert!(check_value_pattern("123-45-6789").is_phi);
        assert!(check_value_pattern("123456789").is_phi);
    }

    #[test]
    fn test_us_zip_detection() {
        assert!(check_value_pattern("12345").is_phi);
        assert!(check_value_pattern("12345-6789").is_phi);
    }

    #[test]
    fn test_canada_postal_detection() {
        assert!(check_value_pattern("K1A 0B1").is_phi);
        assert!(check_value_pattern("M5V3L9").is_phi);
    }

    #[test]
    fn test_long_id_detection() {
        assert!(check_value_pattern("ABC123DEF456").is_phi);
        assert!(check_value_pattern("Patient12345").is_phi);
        assert!(check_value_pattern("A1B2C3D4E5F6").is_phi);
    }

    #[test]
    fn test_long_id_letters_only_not_phi() {
        // All letters - not suspicious
        assert!(!check_value_pattern("ABCDEFGHIJKL").is_phi);
    }

    #[test]
    fn test_long_id_digits_only_not_phi() {
        // All digits - could be legitimate numeric ID
        assert!(!check_value_pattern("123456789012").is_phi);
    }

    #[test]
    fn test_safe_values() {
        assert!(!check_value_pattern("42").is_phi);
        assert!(!check_value_pattern("Male").is_phi);
        assert!(!check_value_pattern("Treatment A").is_phi);
        assert!(!check_value_pattern("2024-01-15").is_phi);
        assert!(!check_value_pattern("3.14159").is_phi);
    }

    #[test]
    fn test_empty_value() {
        assert!(!check_value_pattern("").is_phi);
        assert!(!check_value_pattern("   ").is_phi);
    }

    #[test]
    fn test_short_alphanumeric() {
        // Short IDs are usually safe
        assert!(!check_value_pattern("AB12").is_phi);
        assert!(!check_value_pattern("Group1").is_phi);
    }

    // HIPAA #14: URLs
    #[test]
    fn test_url_detection() {
        assert!(check_value_pattern("https://example.com/patient/123").is_phi);
        assert!(check_value_pattern("http://hospital.org/records").is_phi);
    }

    // HIPAA #15: IP addresses
    #[test]
    fn test_ipv4_detection() {
        assert!(check_value_pattern("192.168.1.1").is_phi);
        assert!(check_value_pattern("10.0.0.255").is_phi);
    }

    #[test]
    fn test_ipv6_detection() {
        assert!(check_value_pattern("2001:0db8:85a3:0000:0000:8a2e:0370:7334").is_phi);
    }

    // HIPAA #13: MAC addresses
    #[test]
    fn test_mac_address_detection() {
        assert!(check_value_pattern("00:1A:2B:3C:4D:5E").is_phi);
        assert!(check_value_pattern("00-1A-2B-3C-4D-5E").is_phi);
    }

    // HIPAA #1: Names - value-level detection
    #[test]
    fn test_name_detection() {
        // Single names
        assert!(check_value_pattern("Smith").is_phi);
        assert!(check_value_pattern("John").is_phi);
        assert!(check_value_pattern("Maria").is_phi);
        assert!(check_value_pattern("Tremblay").is_phi);

        // Full names
        assert!(check_value_pattern("Mary Smith").is_phi);
        assert!(check_value_pattern("John Johnson").is_phi);
        assert!(check_value_pattern("Jose Silva").is_phi);

        // Canadian Census names
        assert!(check_value_pattern("Muhammad").is_phi);
        assert!(check_value_pattern("Aaliyah").is_phi);
    }

    #[test]
    fn test_non_names() {
        // Clinical terms should not be detected as names
        assert!(!check_value_pattern("Treatment").is_phi);
        assert!(!check_value_pattern("Control").is_phi);
        assert!(!check_value_pattern("Placebo").is_phi);
        assert!(!check_value_pattern("Baseline").is_phi);
    }
}
