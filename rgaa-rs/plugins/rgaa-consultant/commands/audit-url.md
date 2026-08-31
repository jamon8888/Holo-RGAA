# `/audit-url` — Full-Site Crawl Audit

## Description

Run a full-site crawl via the orchestrator pipeline. Unlike `/audit-site` which analyzes a single URL, `/audit-url` starts from a seed URL and recursively discovers and audits pages up to configured limits.

## Prerequisites

- `rgaa-mcp` MCP server connected (recommended), OR
- `rgaa-cli` installed (`cargo install --path rgaa-rs/crates/rgaa-cli`)

## Usage

```
/audit-url <url> [--max-pages <n>] [--max-depth <n>] [--respect-robots] [--sample-mode]
```

## Arguments

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `url` | string | Yes | Seed URL to start crawling |
| `--max-pages` | integer | No | Maximum pages to crawl (default: 50) |
| `--max-depth` | integer | No | Maximum crawl depth (default: 5) |
| `--respect-robots` | flag | No | Honor robots.txt directives (default: true) |
| `--sample-mode` | flag | No | Sample mode — audit a subset (default: false) |

## Examples

```
/audit-url https://example.gouv.fr
/audit-url https://example.gouv.fr --max-pages 100
/audit-url https://example.gouv.fr --max-depth 3 --sample-mode
```

## Workflow

1. **Validate URL** — Check format and seed reachability
2. **Build crawl frontier** — Discover links up to max_pages / max_depth
3. **Run audits** — Each page runs `analyze` with shared config
4. **Aggregate results** — Combine findings across all pages
5. **Compute compliance** — Taux Global across the entire site
6. **Store audit** — Returns audit_id for later retrieval

## Output

```
RGAA Full-Site Audit
══════════════════════════════════════════
URL:        https://example.gouv.fr
Pages:      47 crawled, 3 failed
Max Depth:  5
Taux Global: 74.1%
Status:     Partiellement Conforme
────────────────────────────
Passed:     78 criteria (across all pages)
Failed:     16 criteria
Needs Review: 12 criteria
Audit ID:   aud_abc123
══════════════════════════════════════════
```

## Retrieve Results

After the audit:
- `/get-audit <audit-id>` — Retrieve full audit via `get_audit_result` MCP tool
- `/generate-report --input <audit-id>` — Generate a formatted report

## Error Codes

| Code | Meaning |
|------|---------|
| 2 | Invalid URL |
| 3 | Browser/execution failure |
| 4 | Seed URL unreachable |
| 5 | Partial crawl (some pages failed) |
