# ert-manifest

Sharing data is cumbersome. Good tools exist to anonymize data, including ARX (https://arx.deidentifier.org/downloads/).

Goal: To create a simple easy-to-distribute program that merely takes the data and returns a data dictionary in JSON.

Another personal project in my attempt to learn Rust and vibe-code responsabilly with Claude Code. Not validated in anyway. Not guaranteed or approved for use.

Final goal is to produce a privacy-preserving metadata extraction from clinical trial data files for federated analysis.

## Overview

ert-manifest extracts structural metadata from data files (CSV, TSV, Excel) while protecting patient privacy. It generates a JSON manifest containing column names, data types, statistics, and unique values—with automatic detection and suppression of Protected Health Information (PHI).

Designed for clinical trial data sharing workflows where sites need to describe their data without exposing sensitive information.

## Features

- **PHI Detection**: Automatically identifies and suppresses columns containing names, MRNs, SSNs, addresses, phone numbers, emails, and other identifiers
- **Multilingual Support**: Recognizes PHI patterns in English, French (Quebec), and Portuguese (Brazil)
- **Site Recoding**: Anonymizes site-identifying values (hospital names, site codes) while preserving them for analysis
- **K-Anonymity**: Suppresses unique values that appear fewer than k times (default k=5)
- **Count Bucketing**: Reports counts as ranges (e.g., "101-1000") rather than exact values
- **Streaming Processing**: Handles large files with O(1) memory using Welford's algorithm and P² quantile estimation
- **File Integrity**: Computes SHA-256 hash for data provenance

## Installation

### From Source

```bash
git clone https://github.com/yourusername/ert-manifest.git
cd ert-manifest
cargo build --release
```

Binary will be at `target/release/ert-manifest`

## Usage

### GUI Mode (Default)

```bash
ert-manifest
```

Launches a drag-and-drop interface. Drop a data file or click "Browse" to select one.

### CLI Mode

```bash
# Output to stdout
ert-manifest scan --input data.csv

# Output to file
ert-manifest scan --input data.csv --out manifest.json

# Adjust k-anonymity threshold
ert-manifest scan --input data.csv --k 10
```

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `--input`, `-i` | Input file path | required |
| `--out`, `-o` | Output JSON path | stdout |
| `--k` | K-anonymity threshold | 5 |
| `--bucket-counts` | Bucket counts into ranges | true |
| `--hash-file` | Compute SHA-256 hash | true |
| `--relaxed` | Enable exact counts/median | false |

## Output Format

```json
{
  "version": "1.0.0",
  "file_name": "trial_data.csv",
  "file_hash": "a1b2c3...",
  "format": "csv",
  "sheets": [
    {
      "name": "trial_data.csv",
      "index": 0,
      "row_count": { "type": "ShortString", "value": ">1000" },
      "columns": [
        {
          "name": { "type": "ShortString", "value": "age" },
          "index": 0,
          "dtype": "integer",
          "classification": "safe",
          "stats": {
            "count": { "type": "ShortString", "value": ">1000" },
            "min": { "type": "Float", "value": 18.0 },
            "max": { "type": "Float", "value": 85.0 },
            "mean": 52.3,
            "median": 54.0
          },
          "unique_values": [...]
        }
      ]
    }
  ]
}
```

## Privacy Protection

### PHI Column Detection

Columns are automatically classified based on name patterns:

| Classification | Action | Examples |
|----------------|--------|----------|
| **PHI** | Values suppressed | `patient_name`, `mrn`, `ssn`, `dob`, `email`, `phone`, `address` |
| **Recode** | Values anonymized | `site`, `hospital`, `clinic`, `facility` |
| **Warning** | Review recommended | `id`, `encounter`, `visit` |
| **Safe** | Values exported | `age`, `treatment_group`, `dose_mg` |

### Supported Languages

- **English**: name, patient, mrn, ssn, dob, address, phone, email...
- **French**: nom, prenom, adresse, courriel, nas, nam, ramq...
- **Portuguese**: nome, cpf, rg, endereco, telefone, sus, prontuario...

### Site Recoding

Site-identifying columns are recoded to anonymous labels:

```
Original: "Vancouver General", "Calgary Foothills", "Vancouver General"
Recoded:  "Site_A", "Site_B", "Site_A"
```

A sidekick file (`*.recode.txt`) is generated for the site to keep the mapping:

```
# ERT-Manifest Recode Mapping
# CONFIDENTIAL - Keep this file secure at your site

## Column 5: site_code

Site_A = Vancouver General
Site_B = Calgary Foothills
```

### Value-Level Protection

Individual values are checked for PHI patterns:
- Email addresses
- Phone numbers (US/Canada)
- Social Security Numbers
- ZIP/Postal codes
- Long alphanumeric identifiers

## Supported Formats

| Format | Extensions |
|--------|------------|
| CSV | `.csv` |
| TSV | `.tsv`, `.tab` |
| Excel | `.xlsx`, `.xls`, `.xlsm`, `.xlsb` |

## Performance

- 100,000 rows: ~0.4 seconds
- 200,000 rows: ~1.8 seconds
- Memory: O(1) for statistics (streaming algorithms)

## Development

```bash
# Run tests
cargo test

# Build release
cargo build --release
```

## License

MIT
