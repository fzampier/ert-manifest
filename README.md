# ERT-Manifest

Privacy-preserving metadata extraction from data files for clinical trial analysis.

## Features

- **Privacy by Default**: Automatic PHI detection, count bucketing, k-anonymity
- **Streaming Statistics**: Welford's algorithm and P² median estimation (O(1) memory)
- **Type Inference**: Automatic detection of integer, numeric, date, boolean, string types
- **Multiple Formats**: CSV, TSV, Excel (.xlsx/.xls)
- **Dual Interface**: Command-line and graphical user interface

## Installation

```bash
cargo build --release
```

## Quick Start

### Command Line

```bash
# Scan a file
ert-manifest scan --input data.csv --out manifest.json

# With custom k-anonymity threshold
ert-manifest scan --input data.csv -k 10
```

### GUI

```bash
ert-manifest gui
```

Drag and drop files or use the file browser.

## Privacy Features

| Feature | Description |
|---------|-------------|
| PHI Column Detection | Detects columns named `name`, `ssn`, `dob`, `email`, etc. |
| PHI Value Detection | Detects emails, phone numbers, SSNs, postal codes |
| Count Bucketing | Replaces exact counts with ranges (e.g., "21-100") |
| K-Anonymity | Only exports values appearing ≥k times (default: 5) |
| Length Limit | Suppresses strings longer than 32 characters |

## Output Example

```json
{
  "version": "1.0.0",
  "file_name": "data.csv",
  "file_hash": "sha256...",
  "format": "csv",
  "sheets": [{
    "name": "data.csv",
    "row_count": {"type": "ShortString", "value": "101-1000"},
    "columns": [{
      "name": {"type": "ShortString", "value": "age"},
      "dtype": "integer",
      "classification": "safe",
      "stats": {
        "count": {"type": "ShortString", "value": "101-1000"},
        "min": {"type": "Float", "value": 18.0},
        "max": {"type": "Float", "value": 85.0},
        "mean": 45.2,
        "std_dev": 12.5,
        "median": 44.0
      }
    }]
  }]
}
```

## CLI Options

```
ert-manifest scan [OPTIONS] --input <FILE>

Options:
  -i, --input <FILE>    Input file path
  -o, --out <FILE>      Output JSON file (stdout if omitted)
  -k <N>                K-anonymity threshold [default: 5]
  --bucket-counts       Bucket counts into ranges [default: true]
  --exact-counts        Use exact counts (requires --relaxed)
  --exact-median        Use exact median (requires --relaxed)
  --hash-file           Compute SHA-256 hash [default: true]
  --relaxed             Enable relaxed mode for non-sensitive data
```

## Documentation

See [MANUAL.md](MANUAL.md) for comprehensive documentation.

## License

MIT
