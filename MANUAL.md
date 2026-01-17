# ERT-Manifest User Manual

## Overview

ERT-Manifest is a privacy-preserving metadata extraction tool designed for clinical trial data analysis. It extracts schema information, statistics, and metadata from data files (CSV, TSV, Excel) while protecting sensitive information through automatic PHI detection, value suppression, and count bucketing.

## Table of Contents

1. [Installation](#installation)
2. [Quick Start](#quick-start)
3. [Command Line Interface](#command-line-interface)
4. [Graphical User Interface](#graphical-user-interface)
5. [Output Format](#output-format)
6. [Privacy Features](#privacy-features)
7. [Type Inference](#type-inference)
8. [Statistics](#statistics)
9. [Supported File Formats](#supported-file-formats)
10. [Configuration Options](#configuration-options)
11. [Examples](#examples)
12. [Troubleshooting](#troubleshooting)

---

## Installation

### From Source

```bash
# Clone the repository
git clone <repository-url>
cd ert-manifest

# Build release version
cargo build --release

# The binary will be at ./target/release/ert-manifest
```

### Requirements

- Rust 1.70 or later
- For GUI: Platform-specific graphics libraries (automatically handled by eframe)

---

## Quick Start

### Command Line

```bash
# Scan a CSV file and output to stdout
ert-manifest scan --input data.csv

# Scan and save to a file
ert-manifest scan --input data.csv --out manifest.json

# Launch the GUI
ert-manifest gui
```

### GUI

1. Run `ert-manifest` or `ert-manifest gui`
2. Drag and drop a data file onto the window, or click "Browse..."
3. View the generated manifest
4. Copy to clipboard or save to file

---

## Command Line Interface

### Commands

#### `scan`

Scan a data file and extract privacy-preserving metadata.

```bash
ert-manifest scan [OPTIONS] --input <FILE>
```

**Options:**

| Option | Description | Default |
|--------|-------------|---------|
| `-i, --input <FILE>` | Input file path (required) | - |
| `-o, --out <FILE>` | Output JSON file path | stdout |
| `-k <N>` | K-anonymity threshold | 5 |
| `--bucket-counts` | Bucket counts into ranges | true |
| `--exact-counts` | Use exact counts (requires --relaxed) | false |
| `--exact-median` | Use exact median (requires --relaxed) | false |
| `--hash-file` | Compute SHA-256 file hash | true |
| `--relaxed` | Enable relaxed mode | false |

#### `gui`

Launch the graphical user interface.

```bash
ert-manifest gui
```

#### `help`

Show help information.

```bash
ert-manifest help
ert-manifest help scan
```

---

## Graphical User Interface

The GUI provides an intuitive interface for processing data files:

### Main Window

1. **Drag-and-Drop Zone**: Drop CSV, TSV, or Excel files here
2. **Browse Button**: Open a file picker dialog
3. **Options Panel** (collapsible):
   - K-anonymity slider (1-20)
   - Bucket counts toggle
   - Compute file hash toggle
   - Relaxed mode toggle
   - Exact counts/median toggles (enabled when relaxed mode is on)

### Results View

After processing:
- **Warnings Panel**: Shows detected PHI columns and other warnings
- **JSON Output**: Full manifest in scrollable text area
- **Copy to Clipboard**: Copy the JSON to clipboard
- **Save to File**: Save as .json file
- **New File**: Reset and process another file

---

## Output Format

The manifest is output as JSON with the following structure:

```json
{
  "version": "1.0.0",
  "file_name": "data.csv",
  "file_hash": "sha256...",
  "format": "csv",
  "sheets": [...],
  "warnings": [...],
  "options": {...}
}
```

### Sheet Schema

Each sheet (or single file for CSV/TSV) contains:

```json
{
  "name": "Sheet1",
  "index": 0,
  "row_count": {"type": "ShortString", "value": "101-1000"},
  "columns": [...],
  "warnings": []
}
```

### Column Schema

Each column contains:

```json
{
  "name": {"type": "ShortString", "value": "age"},
  "index": 0,
  "dtype": "integer",
  "classification": "safe",
  "stats": {
    "count": {"type": "ShortString", "value": "101-1000"},
    "missing_count": {"type": "ShortString", "value": "0"},
    "min": {"type": "Float", "value": 18.0},
    "max": {"type": "Float", "value": 85.0},
    "mean": 45.2,
    "std_dev": 12.5,
    "median": 44.0,
    "unique_count": {"type": "ShortString", "value": "21-100"}
  },
  "unique_values": [...],
  "warnings": []
}
```

### SafeValue Types

Values are wrapped in privacy-safe containers:

| Type | Description |
|------|-------------|
| `Integer` | Exact integer value |
| `Float` | Exact floating-point value |
| `Boolean` | True/false value |
| `ShortString` | String value (≤32 characters, passes safety checks) |
| `Suppressed` | Value suppressed with reason |

---

## Privacy Features

### PHI Column Detection

Column names are checked against patterns that may indicate Protected Health Information:

**Auto-suppressed patterns** (values are hidden):
- `name`, `patient`, `subject_name`
- `mrn`, `medical_record`
- `ssn`, `social_security`
- `dob`, `birth`, `birthday`
- `address`, `street`, `city`, `zip`, `postal`
- `phone`, `email`, `contact`
- `provider`, `physician`, `nurse`, `doctor`
- `site`, `hospital`, `clinic`, `facility`

**Warning-only patterns** (flagged but not suppressed):
- `id`, `identifier`, `code`, `number`

### PHI Value Detection

Values are checked against regex patterns:

| Pattern | Example |
|---------|---------|
| Email | `user@example.com` |
| US Phone | `555-123-4567`, `(555) 123-4567` |
| SSN | `123-45-6789` |
| US ZIP | `12345`, `12345-6789` |
| Canada Postal | `K1A 0B1` |
| Long Alphanumeric ID | `ABC123DEF456` (10+ chars, mixed letters/digits) |

### Count Bucketing

When `--bucket-counts` is enabled (default), exact counts are replaced with ranges:

| Count | Bucket |
|-------|--------|
| 0 | `"0"` |
| 1 | `"1"` |
| 2-5 | `"2-5"` |
| 6-10 | `"6-10"` |
| 11-20 | `"11-20"` |
| 21-100 | `"21-100"` |
| 101-1000 | `"101-1000"` |
| >1000 | `">1000"` |

### K-Anonymity

Unique values are only included in the output if they appear at least `k` times (default: 5). This prevents identification of rare values that might be personally identifiable.

### Value Length Limit

String values longer than 32 characters are automatically suppressed to prevent leakage of free-text data.

### High Cardinality Protection

Columns with more than 2000 unique values are marked as "high cardinality" and their unique values are not exported.

---

## Type Inference

ERT-Manifest uses a two-pass inference algorithm:

### Pass 1: Sample Collection

The first 2000 non-missing values are sampled to determine the initial type.

### Pass 2: Type Validation

During the full scan, types may be upgraded:
- `integer` → `numeric` (if decimals found)
- `numeric` → `string` (if non-numeric found)
- `string` → `free_text` (if many long strings found)

### Detected Types

| Type | Description |
|------|-------------|
| `integer` | Whole numbers |
| `numeric` | Decimal numbers |
| `string` | Short text values |
| `date` | Date values (various formats) |
| `datetime` | Date and time values |
| `boolean` | True/false values |
| `free_text` | Long text or multi-line content |

### Boolean Recognition

The following tokens are recognized as boolean:
- True: `true`, `yes`, `y`, `1`, `t`
- False: `false`, `no`, `n`, `0`, `f`

### Date Format Recognition

Supported date formats:
- ISO: `2024-01-15`
- US: `01/15/2024`, `1/15/2024`
- European: `15-01-2024`
- Short year: `01/15/24`
- Month name: `January 15, 2024`

### Missing Value Detection

The following are treated as missing values:
- Empty string
- `NA`, `N/A`, `na`, `n/a`
- `NULL`, `null`
- `NaN`, `nan`
- `.`, `-`, `--`
- `missing`, `MISSING`
- `None`, `none`
- Excel errors: `#N/A`, `#VALUE!`, `#REF!`, `#DIV/0!`, `#NUM!`, `#NAME?`, `#NULL!`

---

## Statistics

### Numeric Columns

For integer and numeric columns, the following statistics are computed:

| Statistic | Algorithm |
|-----------|-----------|
| Count | Exact count of non-missing values |
| Missing Count | Count of missing values |
| Min | Minimum value |
| Max | Maximum value |
| Mean | Welford's online algorithm |
| Std Dev | Welford's online algorithm |
| Median | P² quantile estimator |

### Welford's Algorithm

Mean and variance are computed in a single pass with O(1) memory using Welford's online algorithm. This is numerically stable even for large datasets.

### P² Quantile Estimator

The median is estimated using the Jain-Chlamtac P² algorithm, which maintains only 5 markers regardless of dataset size. The estimate converges to the true median as more data is processed.

### String Columns

For string columns:
- Count and missing count
- Unique value count (up to 2000)
- List of unique values (if safe to export)

---

## Supported File Formats

### CSV (`.csv`)

- Standard comma-separated values
- Header row required
- Flexible parsing (handles missing columns)

### TSV (`.tsv`, `.tab`)

- Tab-separated values
- Header row required

### Excel (`.xlsx`, `.xls`, `.xlsm`, `.xlsb`)

- All sheets are processed
- First row treated as headers
- Native Excel types preserved (dates, numbers, booleans)
- Error cells treated as missing

### Future Formats (Feature-Gated)

The following formats have stub implementations for future development:
- Stata (`.dta`)
- SAS (`.sas7bdat`)
- SPSS (`.sav`)

---

## Configuration Options

### K-Anonymity (`-k`)

Sets the minimum count required for a value to be included in the unique values list.

```bash
# Require at least 10 occurrences
ert-manifest scan --input data.csv -k 10
```

### Bucket Counts (`--bucket-counts`)

When enabled (default), exact counts are replaced with ranges. Disable with `--bucket-counts=false`.

```bash
# Disable bucketing (not recommended for sensitive data)
ert-manifest scan --input data.csv --bucket-counts=false
```

### Relaxed Mode (`--relaxed`)

Enables `--exact-counts` and `--exact-median` options. Use only when data is not sensitive.

```bash
# Enable exact values
ert-manifest scan --input data.csv --relaxed --exact-counts --exact-median
```

### File Hash (`--hash-file`)

Computes a SHA-256 hash of the input file. Enabled by default.

```bash
# Disable hashing
ert-manifest scan --input data.csv --hash-file=false
```

---

## Examples

### Basic Usage

```bash
# Scan a CSV file
ert-manifest scan --input clinical_data.csv

# Save output to file
ert-manifest scan --input clinical_data.csv --out manifest.json
```

### Custom K-Anonymity

```bash
# Stricter privacy (k=10)
ert-manifest scan --input data.csv -k 10
```

### Exact Values (Non-Sensitive Data)

```bash
# For non-sensitive data, get exact counts and median
ert-manifest scan --input public_data.csv --relaxed --exact-counts --exact-median
```

### Processing Excel Files

```bash
# Multi-sheet Excel file
ert-manifest scan --input workbook.xlsx --out manifest.json
```

### Piping Output

```bash
# Pipe to jq for pretty printing
ert-manifest scan --input data.csv | jq .

# Extract just column names
ert-manifest scan --input data.csv | jq '.sheets[0].columns[].name'
```

---

## Troubleshooting

### "Unsupported file format"

Ensure the file has a recognized extension (`.csv`, `.tsv`, `.xlsx`, `.xls`).

### "Column name matches PHI pattern"

This warning indicates a column name that may contain sensitive data. Review the data and consider renaming non-sensitive columns.

### GUI Won't Start

Ensure your system has graphics drivers installed. On headless systems, use the CLI instead.

### Large Files

ERT-Manifest uses streaming algorithms and should handle large files efficiently. Memory usage is O(columns) not O(rows).

### Excel Date Issues

Excel stores dates as serial numbers. ERT-Manifest converts these to ISO format strings. If dates appear as numbers, ensure the Excel column is formatted as a date.

---

## Version History

### 1.0.0

- Initial release
- CSV, TSV, Excel support
- PHI column and value detection
- Welford and P² statistics
- CLI and GUI interfaces

---

## License

MIT License

---

## Support

For issues and feature requests, please open an issue on the project repository.
