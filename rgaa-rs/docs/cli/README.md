# RGAA CLI Reference

The `rgaa-cli` binary provides a command-line interface for running RGAA accessibility audits, generating reports, and verifying compliance.

## Installation

```bash
cargo install --path crates/rgaa-cli
```

Or use the pre-built binary from the [release page](https://github.com/jamon8888/Holo-RGAA/releases).

## Global Flags

| Flag | Description |
|------|-------------|
| `--config <PATH>` | Path to config file (default: `.rgaa/config.yaml`) |
| `--output <PATH>` | Write output to file instead of stdout |
| `--format <FORMAT>` | Output format: `json`, `markdown`, `sarif`, `junit` |
| `--audit-id <ID>` | Audit ID for operations requiring stored results |

## Commands

### `rgaa audit analyze`

Run an RGAA accessibility audit against a URL or configured profile.

```bash
rgaa audit analyze [OPTIONS]
```

**Options:**

| Flag | Description | Default |
|------|-------------|---------|
| `--url <URL>` | URL to audit | - |
| `--profile <NAME>` | Name of a configured URL profile | - |
| `--format <FORMAT>` | Output format: `json`, `table`, `html` | `json` |
| `--verbose` | Enable detailed progress output | false |

**Examples:**

```bash
# Audit a single URL
rgaa audit analyze --url https://example.test

# Use a configured profile
rgaa audit analyze --profile production

# Table output with verbose progress
rgaa audit analyze --url https://example.test --format table --verbose

# Save output to file
rgaa audit analyze --url https://example.test --output results.json
```

**Exit Codes:**

- `0`: Audit completed successfully
- `1`: Policy failure (non-compliant)
- `2`: Invalid input
- `3`: Execution error (network, browser, etc.)

---

### `rgaa audit igt`

Run a guided accessibility test (interactive browser automation).

```bash
rgaa audit igt --test <TEST_NAME> [OPTIONS]
```

**Options:**

| Flag | Description |
|------|-------------|
| `--test <TEST>` | Name of the guided test to run |
| `--config <PATH>` | Path to config file |

**Example:**

```bash
rgaa audit igt --test keyboard-navigation
```

---

### `rgaa audit verify`

Verify remediation proposals by running them through the remediation engine.

```bash
rgaa audit verify --issues <PATH> [OPTIONS]
```

**Options:**

| Flag | Description |
|------|-------------|
| `--issues <PATH>` | Path to JSON file containing remediation issues |
| `--output <PATH>` | Write output to file |

**Example:**

```bash
rgaa audit verify --issues remediation-issues.json --output verification.txt
```

---

### `rgaa audit report`

Render an audit bundle as a formatted report.

```bash
rgaa audit report [OPTIONS]
```

**Options:**

| Flag | Description |
|------|-------------|
| `--input <BUNDLE>` | Path to audit bundle JSON file |
| `--format <FORMAT>` | Report format: `json`, `markdown`, `sarif`, `junit`, `html` |
| `--output <PATH>` | Write output to file |

**Examples:**

```bash
# Generate HTML report
rgaa audit report --input bundle.json --format html --output report.html

# Generate SARIF for GitHub Code Scanning
rgaa audit report --input bundle.json --format sarif --output results.sarif
```

---

### `rgaa audit policy`

Check compliance against configured policy thresholds.

```bash
rgaa audit policy --input <BUNDLE> [OPTIONS]
```

**Exit Codes:**

- `0`: Compliant
- `1`: Non-compliant

**Example:**

```bash
rgaa audit policy --input audit-bundle.json
# Output: compliance 85.50% (minimum 80.00%): PASS
```

---

## Configuration File

`.rgaa/config.yaml`:

```yaml
url_profiles:
  default:
    url: https://example.test
  production:
    url: https://myapp.com

viewport_profiles:
  desktop:
    width: 1280
    height: 720
  mobile:
    width: 375
    height: 667

policy:
  min_compliance: 80.0
  required_criteria:
    - "1.1"
    - "5.1"

guided_tests:
  - keyboard-navigation
  - focus-visibility
```

---

## Output Formats

### JSON

```json
{
  "audit_id": "aud_abc123",
  "url": "https://example.test",
  "taux_global": 85.5,
  "coverage_percent": 92.3,
  "etat_conformite": "partielle",
  "passed": 45,
  "failed": 8,
  "na": 53,
  "total_criteria": 106,
  "pages": [...]
}
```

### SARIF / JUnit XML

For CI integration.

### HTML

Styled report suitable for sharing.

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `HOLO3_API_KEY` | API key for LLM-assisted evaluation |
| `RGAA_EVIDENCE_DIR` | Override evidence directory |
| `OBSCURA_BIN` | Path to Obscura browser binary |

---

## Error Handling

| Exit Code | Type | Description |
|-----------|------|-------------|
| `0` | Success | Operation completed successfully |
| `1` | Policy Failure | Compliance check failed |
| `2` | Invalid Input | User-provided parameters are invalid |
| `3` | Execution Error | System-level failure |
