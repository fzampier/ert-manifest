use crate::types::Classification;

/// Patterns that indicate PHI in column names (suppress values)
const PHI_PATTERNS: &[&str] = &[
    // ===== NAMES (English) =====
    "name",
    "patient",
    "subject",       // catches subj_nm, subject_id context
    "first_name",
    "last_name",
    "fname",
    "lname",
    "surname",
    "given_name",
    "initials",
    // ===== NAMES (French) =====
    "nom",           // name
    "nom_famille",   // family name
    "prenom",        // first name
    // ===== NAMES (Portuguese - Brazil) =====
    "nome",          // name
    "nome_paciente", // patient name
    "sobrenome",     // surname
    // ===== MEDICAL RECORD NUMBERS =====
    "mrn",
    "medical_record",
    "chart",
    "chart_number",
    // ===== CANADIAN HEALTH IDENTIFIERS =====
    "phn",           // Personal Health Number (BC, AB, MB, SK)
    "sin",           // Social Insurance Number
    "ohip",          // Ontario Health Insurance Plan
    "ahcip",         // Alberta Health Care Insurance Plan
    "msp",           // Medical Services Plan (BC)
    "healthcard",
    "health_card",
    "care_card",
    // ===== QUEBEC HEALTH IDENTIFIERS (French) =====
    "nas",           // Numéro d'assurance sociale (SIN in French)
    "nam",           // Numéro d'assurance maladie (RAMQ)
    "numero_assurance_maladie",
    "ramq",          // Régie de l'assurance maladie du Québec
    // ===== BRAZILIAN IDENTIFIERS (Portuguese) =====
    "cpf",           // Cadastro de Pessoas Físicas (Brazilian SSN - CRITICAL)
    "rg",            // Registro Geral (ID card number)
    "sus",           // Sistema Único de Saúde (public health)
    "cartao_sus",    // SUS card
    "cns",           // Cartão Nacional de Saúde
    "prontuario",    // medical record
    // ===== US IDENTIFIERS =====
    "ssn",
    "social_security",
    // ===== DATES (English) =====
    "dob",
    "birth",
    "birthday",
    "date_of_birth",
    "admission_date",
    "discharge_date",
    "death_date",
    "date_of_death",
    "dod",            // date of death
    // ===== DATES (French) =====
    "naissance",     // birth
    "date_naissance",
    "ddn",           // date de naissance (DOB)
    // ===== DATES (Portuguese - Brazil) =====
    "nascimento",    // birth
    "data_nascimento",
    "dt_nasc",       // abbreviated
    "dn",            // date of birth abbreviated
    // ===== ADDRESS (English) =====
    "address",
    "street",
    "city",
    "zip",
    "postal",
    // ===== ADDRESS (French) =====
    "adresse",
    // ===== ADDRESS (Portuguese - Brazil) =====
    "endereco",      // address
    "municipio",     // municipality/city
    "cidade",        // city
    "cep",           // postal code (ZIP equivalent)
    "uf",            // state abbreviation
    // ===== CONTACT (English) =====
    "phone",
    "email",
    "contact",
    "fax",
    // ===== CONTACT (French) =====
    "courriel",      // email
    "telephone",
    "tel",
    // ===== CONTACT (Portuguese - Brazil) =====
    "telefone",      // phone
    "fone",          // phone (short)
    "cel",           // cell
    "celular",       // cellular
    // ===== EMERGENCY/FAMILY CONTACTS =====
    "kin",
    "next_of_kin",
    "emergency_contact",
    "guarantor",
    // ===== FAMILY (Portuguese - Brazil) =====
    // Mother's name is used for ID verification in Brazil - CRITICAL
    "mae",           // mother
    "nome_mae",      // mother's name
    "pai",           // father
    "nome_pai",      // father's name
    // ===== HEALTHCARE PROVIDERS (English) =====
    "provider",
    "physician",
    "nurse",
    "doctor",
    "attending",
    "resident",
    // ===== HEALTHCARE PROVIDERS (French) =====
    "medecin",       // physician
    "md",            // médecin
    "infirmier",     // nurse (m)
    "infirmiere",    // nurse (f)
    // ===== HEALTHCARE PROVIDERS (Portuguese - Brazil) =====
    "medico",        // physician
    "enfermeiro",    // nurse (m)
    "enfermeira",    // nurse (f)
    // ===== ABBREVIATED FORMS =====
    "pt_",           // pt_name, pt_id
    "_pt",           // patient_pt
    "subj",          // subj_id, subj_name
    // ===== HIPAA #9: HEALTH PLAN BENEFICIARY NUMBERS =====
    "insurance",
    "policy",
    "policy_number",
    "beneficiary",
    "member_id",
    "subscriber",
    "group_number",
    "plan_id",
    // ===== HIPAA #10: ACCOUNT NUMBERS =====
    "account",
    "acct",
    "account_number",
    "billing",
    // ===== HIPAA #11: CERTIFICATE/LICENSE NUMBERS =====
    "license",
    "license_number",
    "certificate",
    "cert_number",
    "credential",
    // ===== HIPAA #12: VEHICLE IDENTIFIERS =====
    "vin",
    "vehicle",
    "license_plate",
    "plate_number",
    // ===== HIPAA #13: DEVICE IDENTIFIERS =====
    "serial",
    "serial_number",
    "device_id",
    "imei",
    "udid",
    "mac_address",
    // ===== HIPAA #14: WEB URLs =====
    "url",
    "website",
    "web_address",
    "homepage",
    // ===== HIPAA #15: IP ADDRESSES =====
    "ip_address",
    "ipv4",
    "ipv6",
    // ===== HIPAA #16: BIOMETRIC IDENTIFIERS =====
    "fingerprint",
    "biometric",
    "voiceprint",
    "retina",
    "iris_scan",
    "face_id",
    // ===== HIPAA #17: PHOTOGRAPHS =====
    "photo",
    "photograph",
    "picture",
    "headshot",
    "face_image",
    "portrait",
];

/// Patterns that should be recoded (anonymized but preserved for analysis)
const PHI_RECODE: &[&str] = &[
    // English
    "site",
    "hospital",
    "clinic",
    "facility",
    "center",
    "location",
    // French
    "hopital",       // hospital
    "clinique",      // clinic
    "centre",        // center
    "etablissement", // facility
];

/// Patterns that warrant a warning but don't auto-suppress
const PHI_WARN_ONLY: &[&str] = &[
    "id",
    "identifier",
    "code",
    "number",
    "encounter",     // Could be sequential/identifying
    "visit",         // visit_id could identify
    "admission",     // admission number
    "case",          // case number
];

/// Result of checking a column name for PHI patterns
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnNameResult {
    pub classification: Classification,
    pub matched_pattern: Option<String>,
    pub warning: Option<String>,
}

impl ColumnNameResult {
    pub fn safe() -> Self {
        Self {
            classification: Classification::Safe,
            matched_pattern: None,
            warning: None,
        }
    }

    pub fn phi(pattern: &str) -> Self {
        Self {
            classification: Classification::Phi,
            matched_pattern: Some(pattern.to_string()),
            warning: Some(format!(
                "Column name matches PHI pattern '{}'; values suppressed",
                pattern
            )),
        }
    }

    pub fn recode(pattern: &str) -> Self {
        Self {
            classification: Classification::Recode,
            matched_pattern: Some(pattern.to_string()),
            warning: Some(format!(
                "Column name matches site-identifying pattern '{}'; values will be recoded",
                pattern
            )),
        }
    }

    pub fn warning(pattern: &str) -> Self {
        Self {
            classification: Classification::Warning,
            matched_pattern: Some(pattern.to_string()),
            warning: Some(format!(
                "Column name matches potentially sensitive pattern '{}'; review recommended",
                pattern
            )),
        }
    }
}

/// Check a column name for PHI patterns
pub fn check_column_name(name: &str) -> ColumnNameResult {
    let name_lower = name.to_lowercase();
    let name_normalized = normalize_column_name(&name_lower);

    // Check PHI patterns first (most restrictive - suppress)
    for pattern in PHI_PATTERNS {
        if matches_pattern(&name_normalized, pattern) {
            return ColumnNameResult::phi(pattern);
        }
    }

    // Check recode patterns (anonymize but preserve)
    for pattern in PHI_RECODE {
        if matches_pattern(&name_normalized, pattern) {
            return ColumnNameResult::recode(pattern);
        }
    }

    // Check warning-only patterns
    for pattern in PHI_WARN_ONLY {
        if matches_pattern(&name_normalized, pattern) {
            return ColumnNameResult::warning(pattern);
        }
    }

    ColumnNameResult::safe()
}

/// Normalize a column name for pattern matching
fn normalize_column_name(name: &str) -> String {
    // Replace common separators with underscores
    name.replace(['-', ' ', '.'], "_")
}

/// Check if a normalized name matches a pattern
fn matches_pattern(normalized_name: &str, pattern: &str) -> bool {
    // Handle prefix patterns (e.g., "pt_" matches "pt_name")
    if pattern.ends_with('_') {
        return normalized_name.starts_with(pattern);
    }

    // Handle suffix patterns (e.g., "_pt" matches "col_pt")
    if pattern.starts_with('_') {
        return normalized_name.ends_with(pattern);
    }

    // Check for exact match
    if normalized_name == pattern {
        return true;
    }

    // Check for word boundary matches
    // Pattern appears at start, end, or surrounded by underscores
    let parts: Vec<&str> = normalized_name.split('_').collect();
    for part in parts {
        if part == pattern {
            return true;
        }
    }

    // Check for pattern as substring with boundaries
    let starts_with_pattern = format!("{}_", pattern);
    let ends_with_pattern = format!("_{}", pattern);
    let contains_pattern = format!("_{}_", pattern);

    normalized_name.starts_with(&starts_with_pattern)
        || normalized_name.ends_with(&ends_with_pattern)
        || normalized_name.contains(&contains_pattern)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_phi_match() {
        let result = check_column_name("name");
        assert_eq!(result.classification, Classification::Phi);
        assert_eq!(result.matched_pattern, Some("name".to_string()));
    }

    #[test]
    fn test_phi_with_prefix() {
        let result = check_column_name("patient_name");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_with_suffix() {
        let result = check_column_name("name_first");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_case_insensitive() {
        let result = check_column_name("PATIENT_NAME");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_mrn() {
        let result = check_column_name("mrn");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_ssn() {
        let result = check_column_name("ssn");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_email() {
        let result = check_column_name("email");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_phone() {
        let result = check_column_name("phone_number");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_address() {
        let result = check_column_name("home_address");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_dob() {
        let result = check_column_name("dob");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_birth() {
        let result = check_column_name("date_of_birth");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_warning_id() {
        // Note: "subject_id" now matches PHI because "subject" is a PHI pattern
        // Use a non-PHI column with "id" to test warning
        let result = check_column_name("record_id");
        assert_eq!(result.classification, Classification::Warning);
        assert_eq!(result.matched_pattern, Some("id".to_string()));
    }

    #[test]
    fn test_safe_column() {
        let result = check_column_name("age");
        assert_eq!(result.classification, Classification::Safe);
        assert!(result.matched_pattern.is_none());
    }

    #[test]
    fn test_safe_treatment() {
        let result = check_column_name("treatment_group");
        assert_eq!(result.classification, Classification::Safe);
    }

    #[test]
    fn test_safe_dose() {
        let result = check_column_name("dose_mg");
        assert_eq!(result.classification, Classification::Safe);
    }

    #[test]
    fn test_phi_with_dashes() {
        let result = check_column_name("patient-name");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_with_spaces() {
        let result = check_column_name("patient name");
        assert_eq!(result.classification, Classification::Phi);
    }

    // Canadian health identifiers
    #[test]
    fn test_phi_phn() {
        let result = check_column_name("phn");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_ohip() {
        let result = check_column_name("ohip_number");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_sin() {
        let result = check_column_name("sin");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_healthcard() {
        let result = check_column_name("health_card");
        assert_eq!(result.classification, Classification::Phi);
    }

    // Name variants
    #[test]
    fn test_phi_first_name() {
        let result = check_column_name("first_name");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_last_name() {
        let result = check_column_name("last_name");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_fname() {
        let result = check_column_name("fname");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_surname() {
        let result = check_column_name("surname");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_initials() {
        let result = check_column_name("patient_initials");
        assert_eq!(result.classification, Classification::Phi);
    }

    // Abbreviated forms
    #[test]
    fn test_phi_pt_prefix() {
        let result = check_column_name("pt_name");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_subject_abbreviated() {
        let result = check_column_name("subj_id");
        assert_eq!(result.classification, Classification::Phi);
    }

    // French forms
    #[test]
    fn test_phi_nom_patient() {
        let result = check_column_name("nom_patient");
        assert_eq!(result.classification, Classification::Phi);
    }

    // Emergency contacts
    #[test]
    fn test_phi_next_of_kin() {
        let result = check_column_name("next_of_kin");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_emergency_contact() {
        let result = check_column_name("emergency_contact");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_guarantor() {
        let result = check_column_name("guarantor");
        assert_eq!(result.classification, Classification::Phi);
    }

    // Chart numbers
    #[test]
    fn test_phi_chart() {
        let result = check_column_name("chart_number");
        assert_eq!(result.classification, Classification::Phi);
    }

    // Warning patterns
    #[test]
    fn test_warning_encounter() {
        let result = check_column_name("encounter_id");
        assert_eq!(result.classification, Classification::Warning);
    }

    #[test]
    fn test_warning_visit() {
        let result = check_column_name("visit_id");
        assert_eq!(result.classification, Classification::Warning);
    }

    // Recode patterns (site should still be recoded, not PHI)
    #[test]
    fn test_recode_site() {
        let result = check_column_name("site_code");
        assert_eq!(result.classification, Classification::Recode);
    }

    // ===== FRENCH PATTERNS (Quebec/Sherbrooke) =====

    #[test]
    fn test_phi_nom_famille() {
        let result = check_column_name("nom_famille");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_prenom() {
        let result = check_column_name("prenom");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_adresse() {
        let result = check_column_name("adresse");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_courriel() {
        let result = check_column_name("courriel");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_telephone() {
        let result = check_column_name("telephone");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_naissance() {
        let result = check_column_name("date_naissance");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_ddn() {
        let result = check_column_name("ddn");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_nas() {
        let result = check_column_name("nas");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_nam() {
        let result = check_column_name("nam");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_ramq() {
        let result = check_column_name("numero_ramq");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_medecin() {
        let result = check_column_name("medecin_traitant");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_recode_hopital() {
        let result = check_column_name("hopital");
        assert_eq!(result.classification, Classification::Recode);
    }

    #[test]
    fn test_recode_clinique() {
        let result = check_column_name("clinique");
        assert_eq!(result.classification, Classification::Recode);
    }

    #[test]
    fn test_recode_centre() {
        let result = check_column_name("centre_hospitalier");
        assert_eq!(result.classification, Classification::Recode);
    }

    // ===== BRAZILIAN PATTERNS (Portuguese) =====

    #[test]
    fn test_phi_nome() {
        let result = check_column_name("nome_paciente");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_sobrenome() {
        let result = check_column_name("sobrenome");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_cpf() {
        // CPF is THE critical identifier in Brazil
        let result = check_column_name("cpf");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_rg() {
        let result = check_column_name("rg");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_sus() {
        let result = check_column_name("cartao_sus");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_cns() {
        let result = check_column_name("cns");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_endereco() {
        let result = check_column_name("endereco");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_cep() {
        let result = check_column_name("cep");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_telefone() {
        let result = check_column_name("telefone");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_celular() {
        let result = check_column_name("celular");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_nascimento() {
        let result = check_column_name("data_nascimento");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_dt_nasc() {
        let result = check_column_name("dt_nasc");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_nome_mae() {
        // Mother's name is critical for ID in Brazil
        let result = check_column_name("nome_mae");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_nome_pai() {
        let result = check_column_name("nome_pai");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_medico() {
        let result = check_column_name("medico");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_prontuario() {
        let result = check_column_name("prontuario");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_recode_hospital_pt() {
        // "hospital" is same in Portuguese
        let result = check_column_name("hospital");
        assert_eq!(result.classification, Classification::Recode);
    }

    // ===== HIPAA COMPLETE COVERAGE TESTS =====

    // HIPAA #3: Additional dates
    #[test]
    fn test_phi_admission_date() {
        let result = check_column_name("admission_date");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_discharge_date() {
        let result = check_column_name("discharge_date");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_death_date() {
        let result = check_column_name("date_of_death");
        assert_eq!(result.classification, Classification::Phi);
    }

    // HIPAA #9: Health plan beneficiary numbers
    #[test]
    fn test_phi_insurance() {
        let result = check_column_name("insurance_id");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_policy_number() {
        let result = check_column_name("policy_number");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_beneficiary() {
        let result = check_column_name("beneficiary_id");
        assert_eq!(result.classification, Classification::Phi);
    }

    // HIPAA #10: Account numbers
    #[test]
    fn test_phi_account() {
        let result = check_column_name("account_number");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_billing() {
        let result = check_column_name("billing_id");
        assert_eq!(result.classification, Classification::Phi);
    }

    // HIPAA #11: Certificate/license numbers
    #[test]
    fn test_phi_license() {
        let result = check_column_name("license_number");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_certificate() {
        let result = check_column_name("certificate_id");
        assert_eq!(result.classification, Classification::Phi);
    }

    // HIPAA #12: Vehicle identifiers
    #[test]
    fn test_phi_vin() {
        let result = check_column_name("vin");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_license_plate() {
        let result = check_column_name("license_plate");
        assert_eq!(result.classification, Classification::Phi);
    }

    // HIPAA #13: Device identifiers
    #[test]
    fn test_phi_serial_number() {
        let result = check_column_name("serial_number");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_device_id() {
        let result = check_column_name("device_id");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_imei() {
        let result = check_column_name("imei");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_mac_address() {
        let result = check_column_name("mac_address");
        assert_eq!(result.classification, Classification::Phi);
    }

    // HIPAA #14: Web URLs
    #[test]
    fn test_phi_url() {
        let result = check_column_name("profile_url");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_website() {
        let result = check_column_name("website");
        assert_eq!(result.classification, Classification::Phi);
    }

    // HIPAA #15: IP addresses
    #[test]
    fn test_phi_ip_address() {
        let result = check_column_name("ip_address");
        assert_eq!(result.classification, Classification::Phi);
    }

    // HIPAA #16: Biometric identifiers
    #[test]
    fn test_phi_fingerprint() {
        let result = check_column_name("fingerprint");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_biometric() {
        let result = check_column_name("biometric_data");
        assert_eq!(result.classification, Classification::Phi);
    }

    // HIPAA #17: Photographs
    #[test]
    fn test_phi_photo() {
        let result = check_column_name("patient_photo");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_photograph() {
        let result = check_column_name("photograph");
        assert_eq!(result.classification, Classification::Phi);
    }

    #[test]
    fn test_phi_headshot() {
        let result = check_column_name("headshot");
        assert_eq!(result.classification, Classification::Phi);
    }
}
