# ert-manifest: Build Specification for Claude Code

> **ert-manifest**: A user-invoked tool that produces a dataset manifest (metadata only), comparable to a data dictionary, with additional privacy-preserving summaries. No row-level data is ever exported.

## Overview

Build a Rust application called `ert-manifest` that extracts dataset structure/metadata from data files without exposing actual patient data. This is the first stage of a two-stage federated analysis system for clinical trial data.

**Purpose:** Produce a dataset manifest (metadata only), comparable to a data dictionary, with additional privacy-preserving summaries. This allows clinical trial data holders to share dataset structure with analysts so analysts can build a custom analysis tool (ert-probe) that runs locally at the data holder site. No row-level data is ever exported.

---

## Core Requirements

### Input
- CSV files (.csv)
- TSV files (.tsv)
- Excel files (.xlsx, .xls) - all sheets
- Stata files (.dta)
- SAS files (.sas7bdat)
- SPSS files (.sav)
- Drag-and-drop or file picker

### Output
- JSON schema file saved to same directory as input
- Filename: `{original_filename}_schema.json`

### CLI (Behind the GUI)

Implement CLI for explicit, testable behavior:

```bash
# Basic usage (subcommand style - canonical)
ert-manifest scan --input <path> [--out <path>]

# With options
ert-manifest scan --input data.csv --k 20 --bucket-counts --median approx

# Relaxed mode
ert-manifest scan --input data.csv --exact-counts --exact-median --k 10

# With file hash for audit trail
ert-manifest scan --input data.csv --hash-file
```

**Default flags:**
- `--k 20` (suppression threshold)
- `--bucket-counts` (default: true)
- `--export-categorical-values safe_only`
- `--median approx` (streaming P² estimator)

**Optional flags:**
- `--exact-counts` (disable bucketing)
- `--exact-median` (use exact calculation, memory-intensive)
- `--hash-file` (include SHA-256 of input in output)
- `--relaxed` (shorthand for --exact-counts --exact-median --k 10)

**CRITICAL: What `--relaxed` does NOT disable:**
- PHI column name warnings (always on)
- PHI value pattern sniffing (always on)
- `SchemaWriter` guard / `SafeValue` type restriction (always on)
- High-cardinality string suppression (always on)

`--relaxed` only affects counts/median precision and k threshold. It never allows raw row export or arbitrary string dump.

---

## What to Extract (Safe - No PHI)

For each column, extract:

1. **Column name** (string)
2. **Inferred type**: one of `integer`, `numeric`, `string`, `date`, `datetime`, `boolean`, `free_text`
3. **Row count** (bucketed by default)
4. **Missing count** (bucketed by default)
5. **Unique count** (bucketed by default)
6. **For low-cardinality categoricals (≤10 unique values):**
   - List of unique values (only if passes all safety checks - see Privacy Safeguards)
   - Count per value (bucketed by default)
   - If any check fails: `"exported_values": false`, `"suppression_reason": "..."`
7. **For numerics:**
   - Min, max (suppressed if n_rows < k)
   - Mean (always safe)
   - Median (approximate by default using P² quantile estimator)
   - No individual values
8. **For dates/datetimes:**
   - Min date, max date (suppressed if n_rows < k)
   - No individual values
9. **For high-cardinality strings (>10 unique values):**
   - Mark as `free_text`
   - Report only: unique count (bucketed), missing count (bucketed)
   - **Do NOT export any actual values**

### Streaming Statistics (Memory Efficient)

For large files:
- **Mean/variance**: Welford's online algorithm (O(1) memory)
- **Min/max**: Trivial streaming
- **Missing**: Counter
- **Unique count**: Capped HashSet (max ~2000 entries), then mark "high_cardinality" and stop tracking
  - **Tri-state output:**
    ```json
    "unique_count_bucketed": ">1000",
    "unique_count_capped": true,
    "unique_count_note": "Tracking capped at 2000; true cardinality >= 2000"
    ```
  - If NOT capped: `"unique_count_capped": false`, no note
- **Categorical counts**: HashMap only while cardinality stays ≤10, otherwise drop values
- **Median**: P² quantile estimator (O(1) memory) by default
  - `--exact-median` flag for exact calculation (only when n_rows ≤ 2,000,000)
  - **Precision note**: Always include in output:
    ```json
    "median": 64.0,
    "median_method": "p2_approx",
    "median_ci": null,
    "median_note": "Approximate; do not cite for publication"
    ```

### File Hash (Optional, Auditable)

Add `--hash-file` flag to include SHA-256 hash of input file in output header:
```json
{
  "source_file": "trial_data.csv",
  "source_file_sha256": "a3f2c8...",
  ...
}
```
Useful for audit trails. Does not leak PHI.

**Implementation note:** Compute hash via streaming read (don't load entire file into memory). For large Excel files, this is critical.

### Type Inference (Two-Pass)

1. Sample first N non-missing values (default N=2000)
2. Infer initial type
3. Upgrade types conservatively during full scan:
   - `integer` → `numeric` if decimal appears
   - `numeric` → `string` if parse fails often (>5%)
   - `string` → `free_text` if high cardinality OR average length > 50 chars
4. Dates: try small set of common formats; if ambiguous, prefer `string`

### Missing Value Definitions (Per Format)

**CSV/TSV:**
- Missing if empty after trimming
- Missing if literal token (case-insensitive): `"NA"`, `"N/A"`, `"NULL"`, `"."`, `""`
- Configurable; defaults on

**Excel:**
- Missing if empty cell
- Missing if blank string
- Missing if error cell (`#N/A`, `#VALUE!`, etc.)

**ReadStat (Stata/SAS/SPSS):**
- Respect file's tagged missing values
- Respect system missing (`.` in Stata, `.A`-`.Z`, etc.)

**Include in output header:**
```json
"missing_tokens": ["", "NA", "N/A", "NULL", "."]
```

### Boolean Parsing (Case-Insensitive, Trimmed)

**True tokens:** `true`, `t`, `yes`, `y`, `1`
**False tokens:** `false`, `f`, `no`, `n`, `0`

If a column contains ONLY these tokens (after trimming, case-insensitive), infer as `boolean`.

### Excel Date Handling (Important)

Calamine often returns `DataType::Float` for Excel dates (Excel stores dates as serial numbers). Handle this:

1. If column is "mostly numeric" with values in Excel date serial range (1-2958465, i.e., 1900-01-01 to 9999-12-31)
2. AND cell formatting indicates date (if calamine exposes it)
3. → Treat as `date` or `datetime`

**Conservative fallback**: If calamine doesn't expose formatting reliably, keep as `numeric` unless explicit string parsing confirms date format. Document this in output:
```json
"dtype": "numeric",
"possible_date": true,
"note": "Values in Excel date range; verify format"
```

---

## Privacy Safeguards (Critical)

### Default Privacy Mode

All exports use conservative privacy defaults. Add CLI flag `--relaxed` to loosen (for non-PHI datasets).

**Default settings:**
```json
"privacy": {
  "k": 20,
  "counts": "bucketed",
  "export_categorical_values": "safe_only",
  "median_method": "p2_approx"
}
```

### K-Anonymity Style Suppression

**Do NOT export categorical values by default.** Only export if ALL conditions pass:

1. `n_rows >= k` (default k=20)
2. No cell count < k (suppress small cells)
3. Column name is not PHI-risk flagged
4. Column type is `integer`, `numeric`, `boolean`, OR short string (max length ≤ 32 chars)
5. For strings: low entropy (not free text patterns)
6. Values do not match PHI patterns (see Value-Based PHI Sniffing)

**If any condition fails:**
- Do NOT export `values` array
- Export only: `unique_count_bucketed`, `missing_count_bucketed`
- Add `"exported_values": false`
- Add `"suppression_reason": "..."` explaining why

**This preserves:**
- `treatment: 0/1` ✓
- `death_28d: 0/1` ✓
- `sex: M/F` ✓

**This blocks:**
- `operator_name: Smith/Jones` ✗
- `site_code: UAB001` (if count < k) ✗
- `notes: [any free text]` ✗

### Short String Safety Heuristics (Concrete Rules)

Even short strings can be identifiers (e.g., `AB1234`). For string categories to be exportable, ALL must be true:

1. Max length ≤ 32 characters
2. At least 80% of values match one of:
   - `^[0-9]+$` (pure numeric codes)
   - `^(yes|no|y|n|true|false|0|1|male|female|m|f)$` (boolean-ish, case-insensitive)
   - `^[A-Za-z]{1,20}$` (single word, letters only) — BUT block if column name includes provider/site/name/hospital/physician/nurse
3. Block if >30% of values contain both letters AND digits (ID-like patterns)
4. Block if any value matches PHI patterns (email, phone, postal)

If any condition fails → suppress values, export only bucketed counts.

### Bucketed Counts (Default)

Never export exact counts by default. Use buckets:
- `"0"` (explicit zero)
- `"1"`
- `"2-5"`
- `"6-10"`
- `"11-20"`
- `"21-100"`
- `"101-1000"`
- `">1000"`

**Centralized Bucketing Function** (implement in `privacy/bucketing.rs`):
```rust
fn bucket_count(n: u64) -> &'static str {
    match n {
        0 => "0",
        1 => "1",
        2..=5 => "2-5",
        6..=10 => "6-10",
        11..=20 => "11-20",
        21..=100 => "21-100",
        101..=1000 => "101-1000",
        _ => ">1000",
    }
}
```
All count bucketing MUST use this single function for consistency. Stage-2 tools will parse these exact strings.

CLI flag `--exact-counts` enables exact counts (use only for non-sensitive data).

### Small Dataset Protection

If `n_rows < k` (default 20):
- Do not export min/max for dates
- Do not export min/max for numerics
- Do not export median for numerics
- Do not export mean for numerics (can reveal outliers in tiny datasets)
- Export only: `"range_present": true`, `"stats_suppressed": true`

```json
{
  "name": "age",
  "dtype": "numeric",
  "classification": "continuous",
  "stats_suppressed": true,
  "suppression_reason": "n_rows < k; statistics suppressed to protect privacy"
}
```

### Column Name Warnings

If any column name contains these patterns (case-insensitive), add a warning and block value export:
- `name`, `patient`, `subject_name`
- `mrn`, `medical_record`
- `ssn`, `social_security`
- `dob`, `birth`, `birthday`
- `address`, `street`, `city`, `zip`, `postal`
- `phone`, `email`, `contact`
- `provider`, `physician`, `nurse`, `doctor`
- `site`, `hospital`, `clinic`, `facility`
- `id` (warn but don't block - could be de-identified ID)

### Value-Based PHI Sniffing

Even for low-cardinality columns, scan values for PHI patterns. Block value export if detected:
- Email regex: `[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}`
- Phone regex: `\d{3}[-.]?\d{3}[-.]?\d{4}` (and international variants)
- Postal/ZIP: `\d{5}(-\d{4})?` (US), `[A-Z]\d[A-Z]\s?\d[A-Z]\d` (Canada)
- Long alphanumeric IDs: `[A-Za-z0-9]{10,}` with mixed letters/numbers
- Date-like strings in non-date columns

If detected: block value export, add warning with pattern type.

### Never Export
- Actual string values from high-cardinality columns (>10 unique)
- Values from columns that fail PHI checks
- Values from columns with cell counts < k
- Any row-level data
- Any value that could be a direct identifier

### Output Warning Header
Include in JSON output:
```json
{
  "warning": "Review this schema before sharing. Ensure no PHI is present.",
  "phi_risk_columns": ["list", "of", "flagged", "columns"],
  "suppressed_columns": ["columns", "with", "values", "blocked"],
  ...
}
```

---

## Output JSON Structure

```json
{
  "manifest_version": "0.1.0",
  "manifest_build": "abc123f",
  "rust_version": "1.75.0",
  "features_enabled": ["formats-basic"],
  "generated_at": "2026-01-07T12:00:00Z",
  "warning": "Review this schema before sharing. Ensure no PHI is present.",
  "privacy": {
    "k": 20,
    "counts": "bucketed",
    "export_categorical_values": "safe_only",
    "median_method": "p2_approx"
  },
  "missing_tokens": ["", "NA", "N/A", "NULL", "."],
  "phi_risk_columns": [],
  "suppressed_columns": [],
  "source_file": "trial_data.xlsx",
  "source_file_sha256": null,
  "file_type": "excel",
  "sheets": [
    {
      "sheet_name": "Patient Data",
      "sheet_index": 0,
      "total_rows": "101-1000",
      "total_rows_exact": null,
      "total_columns": 15,
      "columns": [
        {
          "name": "subject_id",
          "dtype": "string",
          "unique_count_bucketed": ">1000",
          "unique_count_capped": true,
          "unique_count_note": "Tracking capped at 2000; true cardinality >= 2000",
          "missing_count": "0",
          "classification": "high_cardinality",
          "exported_values": false,
          "phi_warning": "Column name contains 'id' - verify this is de-identified"
        },
        {
          "name": "treatment",
          "dtype": "integer",
          "unique_count_bucketed": "2-5",
          "unique_count_capped": false,
          "missing_count": "0",
          "classification": "categorical",
          "exported_values": true,
          "values": [
            {"value": 0, "count": ">1000"},
            {"value": 1, "count": ">1000"}
          ]
        },
        {
          "name": "death_28d",
          "dtype": "integer",
          "unique_count_bucketed": "2-5",
          "unique_count_capped": false,
          "missing_count": "0",
          "classification": "categorical",
          "exported_values": true,
          "values": [
            {"value": 0, "count": ">1000"},
            {"value": 1, "count": ">1000"}
          ]
        },
        {
          "name": "age",
          "dtype": "numeric",
          "unique_count_bucketed": "21-100",
          "unique_count_capped": false,
          "missing_count": "2-5",
          "classification": "continuous",
          "exported_values": false,
          "stats": {
            "min": 18.0,
            "max": 97.0,
            "mean": 62.4,
            "median": 64.0,
            "median_method": "p2_approx",
            "median_ci": null,
            "median_note": "Approximate; do not cite for publication"
          }
        },
        {
          "name": "rand_date",
          "dtype": "date",
          "unique_count_bucketed": ">1000",
          "unique_count_capped": true,
          "unique_count_note": "Tracking capped at 2000; true cardinality >= 2000",
          "missing_count": "0",
          "classification": "date",
          "exported_values": false,
          "range": {
            "min": "2017-05-29",
            "max": "2020-03-02"
          }
        },
        {
          "name": "operator_name",
          "dtype": "string",
          "unique_count_bucketed": "6-10",
          "unique_count_capped": false,
          "missing_count": "0",
          "classification": "categorical",
          "exported_values": false,
          "suppression_reason": "Column name suggests PHI (contains 'name')",
          "note": "Values not exported due to PHI risk"
        },
        {
          "name": "notes",
          "dtype": "free_text",
          "unique_count_bucketed": ">1000",
          "unique_count_capped": true,
          "unique_count_note": "Tracking capped at 2000; true cardinality >= 2000",
          "missing_count": "101-1000",
          "classification": "free_text_excluded",
          "exported_values": false,
          "note": "High-cardinality text field - values not exported"
        },
        {
          "name": "site_code",
          "dtype": "string",
          "unique_count_bucketed": "2-5",
          "unique_count_capped": false,
          "missing_count": "0",
          "classification": "categorical",
          "exported_values": false,
          "suppression_reason": "Cell count below k threshold",
          "note": "Some categories have fewer than 20 observations"
        }
      ]
    },
    {
      "sheet_name": "Lab Results",
      "sheet_index": 1,
      "total_rows": ">1000",
      "total_columns": 8,
      "columns": []
    }
  ]
}
```

**Note:** For single-sheet formats (CSV, TSV, Stata, SAS, SPSS), the `sheets` array will contain one entry with `sheet_name` set to the filename.

### CLI Flags for Relaxed Mode

```bash
# Default: privacy-preserving
ert-manifest scan --input data.csv

# With file hash for audit
ert-manifest scan --input data.csv --hash-file

# Relaxed: exact counts (for non-sensitive data only)
ert-manifest scan --input data.csv --exact-counts

# Relaxed: exact median (memory-intensive for large files)
ert-manifest scan --input data.csv --exact-median

# Adjust k threshold
ert-manifest scan --input data.csv --k 10

# Full relaxed mode (all exact, lower k) - USE WITH CAUTION
ert-manifest scan --input data.csv --relaxed
```

---

## GUI Requirements (egui)

Simple single-window interface:

```
┌────────────────────────────────────────┐
│           ert-manifest v0.1.0             │
├────────────────────────────────────────┤
│ Privacy: k=20 | Bucketed | Safe-only   │
├────────────────────────────────────────┤
│                                        │
│   Drag data file here                  │
│   (.csv .tsv .xlsx .dta .sas7bdat .sav)│
│                                        │
│          [Or click to browse]          │
│                                        │
├────────────────────────────────────────┤
│ Status: Ready                          │
│                                        │
└────────────────────────────────────────┘
```

### States:
1. **Ready** - waiting for file
2. **Processing** - reading file, show spinner or progress
3. **Done** - show success message, path to output file
4. **Error** - show error message (file not found, parse error, etc.)

### On Success:
- Show: "✓ Schema saved to: /path/to/file_schema.json"
- Show: "Sheets: 2 | Columns: 15 | Rows: 10,520 | Warnings: 2"
- Button: "Open Folder" (optional)

### On PHI Warnings (prominent display):
If any `phi_risk_columns` detected, show warning panel in yellow/orange:
```
⚠ PHI Risk Detected in 3 columns:
  - patient_name (contains 'name')
  - mrn (contains 'mrn')
  - dob (contains 'dob')

Review schema before sharing!
```

### On Warning (PHI risk detected):
- Show warnings prominently in yellow/orange
- List flagged columns
- Still save the file, but make user aware

---

## Dependencies (Suggested)

```toml
[dependencies]
eframe = "0.27"           # egui framework
egui = "0.27"
rfd = "0.14"              # native file dialogs
csv = "1.3"               # CSV/TSV reading
calamine = "0.24"         # Excel reading
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
chrono = "0.4"
regex = "1.10"            # PHI pattern detection
clap = { version = "4.4", features = ["derive"] }  # CLI argument parsing

# Optional: Stata/SAS/SPSS support
readstat = { version = "0.2", optional = true }

[features]
default = ["formats-basic"]
formats-basic = []
formats-readstat = ["readstat"]
```

**Build commands:**
```bash
# Default (CSV/TSV/Excel only) - most portable
cargo build --release

# With Stata/SAS/SPSS support
cargo build --release --features formats-readstat

# Full feature set
cargo build --release --all-features
```

**Note on readstat:** Wraps ReadStat C library. May require system dependencies. If user drops .dta/.sas7bdat/.sav without readstat feature, GUI shows friendly error: "Stata/SAS/SPSS support not included in this build. Use CSV or Excel, or request a build with extended format support."

---

## Build Targets

Compile for:
- Windows (x86_64-pc-windows-msvc)
- macOS (x86_64-apple-darwin, aarch64-apple-darwin)
- Linux (x86_64-unknown-linux-gnu)

Use `--release` and `strip` for minimal binary size.

Target size: **under 20 MB** (readstat adds some weight)

---

## File Structure

```
ert-manifest/
├── Cargo.toml
├── src/
│   ├── main.rs           # Entry point, GUI
│   ├── cli.rs            # CLI argument parsing
│   ├── schema.rs         # Schema extraction logic
│   ├── stats.rs          # Streaming statistics (Welford, P² median)
│   ├── readers/
│   │   ├── mod.rs
│   │   ├── csv.rs        # CSV/TSV reader
│   │   ├── excel.rs      # Excel reader (multi-sheet)
│   │   ├── stata.rs      # Stata .dta reader (feature-gated)
│   │   ├── sas.rs        # SAS .sas7bdat reader (feature-gated)
│   │   └── spss.rs       # SPSS .sav reader (feature-gated)
│   ├── privacy/
│   │   ├── mod.rs
│   │   ├── column_names.rs   # Column name PHI detection
│   │   ├── value_patterns.rs # Value-based PHI sniffing (email, phone, etc.)
│   │   ├── suppression.rs    # K-anonymity suppression logic
│   │   └── bucketing.rs      # Count bucketing
│   ├── types.rs          # Type inference logic
│   └── output.rs         # JSON serialization
├── README.md
└── LICENSE               # MIT
```

---

## Testing

**Core functionality:**
1. Simple CSV (3 columns, 100 rows)
2. TSV file
3. Large CSV (20 columns, 100,000 rows) - verify streaming works
4. Excel file with multiple sheets (extract all sheets)
5. Excel file with single sheet
6. Stata .dta file (with feature flag)
7. SAS .sas7bdat file (with feature flag)
8. SPSS .sav file (with feature flag)
9. File with dates in various formats
10. File with missing values
11. File with mixed column types

**Privacy tests (critical):**
12. File with column names that trigger PHI warnings (name, mrn, dob, etc.)
13. File with email addresses in a column - should detect and suppress
14. File with phone numbers - should detect and suppress
15. File with low-cardinality names (e.g., 5 operator names) - should suppress
16. File with small cells (categories with count < k) - should suppress values
17. Small dataset (n < 20) - should suppress min/max/median
18. File with `treatment: 0/1` - should export values (safe)
19. File with `death_28d: 0/1` - should export values (safe)
20. File with postal codes - should detect and suppress
21. File with long alphanumeric IDs - should detect and suppress

**Edge cases:**
22. Empty file
23. File with only headers
24. File with unicode column names
25. File with very long column names

---

## README.md Content

```markdown
# ert-manifest

A user-invoked tool that produces a dataset manifest (metadata only), comparable to a data dictionary, with additional privacy-preserving summaries. No row-level data is ever exported.

## Supported Formats

- CSV (.csv)
- TSV (.tsv)
- Excel (.xlsx, .xls) - all sheets
- Stata (.dta)
- SAS (.sas7bdat)
- SPSS (.sav)

## Usage

1. Download the appropriate binary for your OS
2. Double-click to run
3. Drag your data file onto the window
4. Schema saved to same folder as your data

## What Gets Exported

- Column names and types
- Row counts
- Summary statistics (min, max, mean for numbers)
- Value counts for categorical variables (≤10 unique values)
- All sheets (for Excel files)

## What Never Gets Exported

- Individual data values from text fields
- Any row-level data
- Anything that could identify a patient

## Privacy

The output JSON should be reviewed before sharing. The tool will warn you if column names suggest potential identifiers (name, MRN, SSN, etc.).

## Part of the e-RT Project

https://github.com/fzampier/ert
```

---

## Notes for Claude Code

### Architecture Invariant (Critical - Prevent Future Leaks)

**Implement a `SchemaWriter` guard** that prevents accidental raw data export:

```rust
// Values array can ONLY hold:
enum SafeValue {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    ShortString(String),  // Only if len <= 32 AND passes safety checks
    Suppressed { reason: String },
}

// SchemaWriter rejects:
// - Any string > 32 chars unless explicitly Suppressed
// - Any value that hasn't passed safety checks
// - Any raw row data
```

This ensures that even if someone adds a feature later, they can't accidentally dump arbitrary strings into the output. The type system enforces the invariant.

### Implementation Guidance (Critical)

**Streaming - Don't Load Entire File:**
- CSV/TSV: Stream rows, never load full file into memory
- Excel: Iterate rows per sheet via calamine; avoid allocating 2D grid
- Large files (>100MB) should work without memory issues

**Per-Column Trackers:**
- Missing counter (simple increment)
- Unique tracker with cap: HashSet up to 2000, then mark "high_cardinality" and stop tracking values
- Numeric stats: Welford algorithm (mean/variance), min/max
- Median: P² quantile estimator (O(1) memory)
- Date range: track min/max

**Type Inference:**
- Infer from first N non-missing values (N=2000)
- Keep adapting during full scan:
  - `integer` → `numeric` if decimal appears
  - `numeric` → `string` if parse fails often (>5%)
  - `string` → `free_text` if high cardinality OR avg length > 50
- Dates: try common formats; if ambiguous, stay `string`

**Safety Checks Order:**
1. Check column name for PHI patterns
2. Check column type eligibility
3. Check n_rows >= k
4. Check all cell counts >= k
5. Check values for PHI patterns (email/phone/etc)
6. Only then export values

### General Notes

1. Start with CSV reader + privacy logic - get that bulletproof first
2. Add Excel multi-sheet support
3. Add GUI after core works
4. Add readstat formats last (feature-gated)
5. Test PHI detection thoroughly - this is the critical path
6. Keep it simple - utility, not product
7. Binary size matters - users will email this

---

## Success Criteria

**Core Functionality:**
- [ ] Reads CSV files correctly (streaming, not full load)
- [ ] Reads TSV files correctly
- [ ] Reads Excel files correctly (all sheets)
- [ ] Reads Stata .dta files correctly (feature-gated)
- [ ] Reads SAS .sas7bdat files correctly (feature-gated)
- [ ] Reads SPSS .sav files correctly (feature-gated)
- [ ] Infers column types accurately
- [ ] Handles Excel dates stored as numbers
- [ ] Exports valid JSON schema
- [ ] Correctly identifies missing values per format
- [ ] Correctly parses boolean tokens

**Privacy (Critical):**
- [ ] Does NOT export categorical string values unless ALL safe-only rules pass
- [ ] Suppresses small cells (count < k)
- [ ] Suppresses min/max/median/mean when n_rows < k
- [ ] Buckets counts by default (with "0" as explicit bucket)
- [ ] Uses centralized `bucket_count()` function everywhere
- [ ] Warns on PHI-risk column names
- [ ] Detects PHI patterns in values (email, phone, postal, IDs)
- [ ] Never exports high-cardinality string values
- [ ] Implements SchemaWriter guard against arbitrary string export
- [ ] Short string heuristics block ID-like patterns
- [ ] `--relaxed` does NOT disable PHI sniffing or SchemaWriter guard
- [ ] Output includes `privacy` block
- [ ] Output includes `suppression_reason` where applicable
- [ ] Output includes `exported_values: true/false` per column
- [ ] Output includes `unique_count_capped: true/false`
- [ ] Median includes precision note ("do not cite")
- [ ] Output includes `missing_tokens` used

**Build & Distribution:**
- [ ] GUI works with drag-and-drop
- [ ] CLI works independently with subcommand format
- [ ] `--hash-file` computes SHA-256 via streaming (not full load)
- [ ] Output includes tool self-identification (version, build, features)
- [ ] ReadStat formats compile behind feature flag
- [ ] Binary under 20 MB
- [ ] Works on Windows, macOS, Linux
- [ ] Handles large files (>100MB) without memory issues

---

## Future Deliverables (Optional)

These can be built after core manifest is working:

1. **JSON Schema** (`schema.schema.json`) - Formal schema definition for output validation
2. **Security/Threat Model** (1 page) - Formal documentation for institutional review
3. **Test Fixture Generator** - Synthetic data files that trigger each privacy rule without real PHI
