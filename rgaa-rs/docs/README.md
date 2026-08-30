# RGAA-RS Documentation

Complete technical and user documentation for the rgaa-rs accessibility auditing platform.

---

## Quick Navigation

| Document | Description |
|----------|-------------|
| [CLI Reference](cli/README.md) | All CLI commands, options, and examples |
| [API Reference](api/README.md) | REST API endpoints and usage |
| [MCP Reference](mcp/README.md) | Model Context Protocol tools |
| [Runbooks](runbooks/) | Step-by-step workflow guides |

---

## Documentation Map

```
docs/
├── README.md           # This file
├── cli/                # CLI reference
│   └── README.md       # All CLI commands
├── api/                # API reference
│   └── README.md       # REST endpoints
├── mcp/                # MCP reference
│   └── README.md       # MCP tools
└── runbooks/           # Workflow guides
    ├── README.md       # Runbooks index
    ├── 01-quickstart.md
    ├── 02-ci-integration.md
    ├── 03-policy-configuration.md
    ├── 04-remediation-workflow.md
    └── 05-guided-testing.md
```

---

## By Role

### For Accessibility Consultants

- [Quick Start](../README.md) — Get running in 5 minutes
- [CLI Reference](cli/README.md) — Full command reference
- [Remediation Workflow](runbooks/04-remediation-workflow.md) — Fix guidance for clients
- [Policy Configuration](runbooks/03-policy-configuration.md) — Per-client compliance rules
- [CI Integration](runbooks/02-ci-integration.md) — Automated audit pipelines

### For Development Teams

- [API Reference](api/README.md) — Integrate with your tools
- [MCP Reference](mcp/README.md) — AI assistant workflows
- [Policy Configuration](runbooks/03-policy-configuration.md) — Enforce compliance in CI
- [Remediation Workflow](runbooks/04-remediation-workflow.md) — Framework-specific fixes
- [Guided Testing](runbooks/05-guided-testing.md) — Manual testing protocols

### For DevOps / Platform Teams

- [CI Integration](runbooks/02-ci-integration.md) — GitHub Actions, GitLab, Jenkins, CircleCI
- [API Reference](api/README.md) — HTTP API deployment
- [Docker Deployment](api/README.md#docker) — Container orchestration
- [Policy Configuration](runbooks/03-policy-configuration.md) — Compliance gates

---

## Core Concepts

### RGAA 4.1.2

The **Referentiel General d'Amelioration de l'Accessibilite** is France's accessibility standard, mandatory for:
- Public sector websites
- Private sector services with public access
- E-commerce platforms

**106 criteria** organized into 13 topics:

| Topic | Criteria | Coverage |
|-------|---------|---------|
| Images | 1.1–1.9 | All image-related requirements |
| Tables | 5.1–5.8 | Data table structure and rendering |
| Links | 6.1–6.3 | Link text and management |
| Scripts | 7.1–7.5 | JavaScript and AJAX accessibility |
| HTML | 8.1–8.10 | Markup quality and language |
| Colors | 10.1–10.14 | Contrast and color dependence |
| Forms | 11.1–11.13 | Form labels and assistance |
| Navigation | 12.1–12.10 | Skip links, focus, focus order |
| Content | 4.1–4.13 | Language and readability |
| Media | 13.1–13.12 | Audio/video and captions |

### Evaluation Tiers

| Tier | Count | Method | Automation |
|------|-------|--------|------------|
| **Deterministe** | 77 | axe-core + gap-fix rules | Fully automated |
| **IA-Assistee** | 22+ | Holo3 LLM evaluation | AI-assisted judgment |
| **PartiellementAutomatable** | 45 | Automated + human review | Hybrid |
| **Manuel** | Remaining | Guided testing | Manual verification |

### Compliance Calculation

**Taux Global (Global Rate):**
```
Taux Global = Conforme / (Conforme + Non Conforme) × 100
```

**Conformity Status:**
- **Conforme** (Compliant): Taux Global > 100% (all criteria pass)
- **Partiellement Conforme** (Partially Compliant): Taux Global ≥ 50%
- **Non Conforme** (Non-Compliant): Taux Global < 50%

### Evidence & Audit Trail

Every audit captures:
- **DOM snapshots** — Full page structure
- **Screenshots** — PNG with SHA-256 hash
- **AXTree dumps** — Accessibility tree
- **Finding fingerprints** — FNV-1a stable IDs for deduplication
- **Evidence references** — Kind, hash, location

---

## Architecture Reference

```
rgaa-orchestrator (Pipeline)
    │
    ├── rgaa-rules (axe-core + gap-fix)
    ├── rgaa-holo (Holo3 LLM)
    ├── rgaa-obscura (Browser CDP)
    │
    └── rgaa-core (Domain Model)
            │
            ├── rgaa-cli
            ├── rgaa-api
            ├── rgaa-mcp
            └── rgaa-remediation
```

---

## Interface Reference

### CLI Commands

| Command | Description |
|---------|-------------|
| `rgaa audit analyze` | Run full RGAA audit |
| `rgaa audit igt` | Run guided accessibility test |
| `rgaa audit verify` | Verify remediation proposals |
| `rgaa audit report` | Render audit bundle as report |
| `rgaa audit policy` | Check compliance against policy |

### API Endpoints

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/audit` | Run new audit |
| `GET` | `/audit/{id}` | Retrieve audit by ID |
| `GET` | `/criteria` | List all 106 criteria |
| `GET` | `/health` | Health check |

### MCP Tools

| Tool | Description |
|------|-------------|
| `analyze` | Analyze URL for accessibility findings |
| `remediate` | Generate remediation guidance |
| `igt` | Run guided accessibility test |
| `audit_url` | Run full RGAA audit |
| `get_audit_result` | Retrieve stored audit |
| `list_criteria` | List all 106 criteria |

---

## Report Formats

| Format | Use Case | Integration |
|--------|----------|-------------|
| **JSON** | Machine processing, custom tooling | Any JSON consumer |
| **Markdown** | Human reports, documentation | GitHub, static sites |
| **SARIF 2.1.0** | GitHub Code Scanning, security dashboards | GitHub, Azure DevOps |
| **JUnit XML** | CI test results, Jenkins, CircleCI | Most CI systems |
| **HTML** | Stakeholder reports, archival | Browser, PDF conversion |

---

## Deployment Reference

### Single Binary

```bash
# Install
curl -sSL https://raw.githubusercontent.com/jamon8888/Holo-RGAA/main/install.sh | bash

# Run
rgaa audit analyze --url https://example.test
```

### Docker

```dockerfile
FROM ghcr.io/jamon8888/rgaa-api:latest
ENV DATABASE_URL=postgres://user:pass@db:5432/rgaa
EXPOSE 3000
CMD ["rgaa-api"]
```

### Claude Code MCP

```json
{
  "mcpServers": {
    "rgaa": {
      "command": "rgaa-mcp",
      "env": { "RGAA_OBSCURA_BIN": "/usr/local/bin/obscura" }
    }
  }
}
```

---

## Troubleshooting

### Common Issues

| Issue | Solution |
|-------|----------|
| `browser unavailable` | Install Obscura and set `RGAA_OBSCURA_BIN` |
| `rate limit exceeded` | Wait and retry; Holo3 has per-minute limits |
| `timeout` | Increase timeout via `--timeout` flag or config |
| `missing alt text` | Add `alt` attributes to images |

### Debug Mode

```bash
# Verbose output
rgaa audit analyze --url https://example.test --verbose

# Debug environment
export RUST_LOG=debug
rgaa audit analyze --url https://example.test
```

---

## Contributing

See [AGENTS.md](../../AGENTS.md) for development guidelines and contribution standards.
