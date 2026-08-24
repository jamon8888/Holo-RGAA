# RGAA Consultant Distribution Solution — Design Spec

**Date:** 2026-08-24
**Status:** Approved
**Approach:** Fat Client (Approach B) — Full-featured local binary + lightweight SaaS backend

---

## 1. Overview

A distribution solution for RGAA accessibility consultants that provides:

- **Single binary** containing all RGAA evaluation, remediation, and reporting tools
- **Claude Desktop / Claude Code integration** via MCP server (stdio transport)
- **CLI interface** for standalone batch audits
- **SaaS backend** for licensing, storage, updates, and billing
- **One-liner install** with automatic Claude Desktop configuration
- **Tiered pricing** with consultant-owned or SaaS-proxied LLM usage

### Target Users

RGAA consultants who perform accessibility audits for French organizations. They work on-site at client locations, often with limited connectivity, and need to produce RGAA compliance reports.

---

## 2. Architecture

### 2.1 System Overview

```
Consultant's Machine                    Your Cloud (SaaS)
┌──────────────────────────────┐       ┌───────────────────────────┐
│ rgaa binary                  │──────▶│ SaaS API (Axum)           │
│                              │       │                           │
│ ┌──────────┐ ┌────────────┐ │       │ ┌───────────────────────┐ │
│ │ rgaa-cli │ │ rgaa-mcp   │ │       │ │ License Service       │ │
│ │ (5 cmds) │ │ (stdio)    │ │       │ │ - Key validation      │ │
│ └────┬─────┘ └─────┬──────┘ │       │ │ - Tier enforcement    │ │
│      │              │        │       │ │ - Offline grace       │ │
│ ┌────▼──────────────▼──────┐ │       │ └───────────────────────┘ │
│ │ rgaa-orchestrator        │ │       │ ┌───────────────────────┐ │
│ │ - Pipeline wiring        │ │       │ │ Audit Storage (PG)    │ │
│ └────┬──────────────┬──────┘ │       │ │ - Result persistence  │ │
│      │              │        │       │ │ - Report generation   │ │
│ ┌────▼─────┐ ┌──────▼─────┐ │       │ └───────────────────────┘ │
│ │ rgaa-    │ │ rgaa-      │ │       │ ┌───────────────────────┐ │
│ │ agent    │ │ obscura    │ │       │ │ Rule Update Feed      │ │
│ │ (LLM)   │ │ (CDP)      │ │       │ │ - axe overrides       │ │
│ └────┬─────┘ └──────┬─────┘ │       │ │ - gap-fix snippets    │ │
│      │              │        │       │ │ - criteria changes    │ │
│ ┌────▼──────────────▼──────┐ │       │ └───────────────────────┘ │
│ │ rgaa-license  (NEW)      │ │◀──────│ ┌───────────────────────┐ │
│ │ rgaa-updater  (NEW)      │ │       │ │ Usage Analytics       │ │
│ └──────────────────────────┘ │       │ │ - Audit counts        │ │
│                              │       │ │ - Billing (Stripe)    │ │
│ Chrome/Chromium (CDP target) │       │ └───────────────────────┘ │
└──────────────────────────────┘       └───────────────────────────┘
```

### 2.2 Crate Composition

The single `rgaa` binary bundles these existing crates:

| Crate | Role | Status |
|-------|------|--------|
| `rgaa-cli` | CLI entrypoint (5 subcommands) | Exists — add `configure` |
| `rgaa-mcp` | MCP server (stdio transport) | Exists — verify Claude Desktop compat |
| `rgaa-orchestrator` | Pipeline wiring | Exists |
| `rgaa-agent` | Rig-based LLM evaluator | Exists |
| `rgaa-holo` | LLM client (multi-provider) | Exists — extend for Claude/OpenAI |
| `rgaa-obscura` | CDP browser automation | Exists |
| `rgaa-core` | Domain types + 106 criteria | Exists |
| `rgaa-rules` | Axe mapper + gap-fix | Exists |
| `rgaa-remediation` | Framework-aware fixes | Exists |
| `rgaa-storage` | Audit persistence | Exists — add SQLite backend |
| `rgaa-license` | License validation + offline grace | **NEW** |
| `rgaa-updater` | Remote rule/config updates | **NEW** |

### 2.3 Key Changes to Existing Crates

**`rgaa-cli`:**
- Add `rgaa configure` subcommand for interactive setup
- Add `rgaa verify-install` to check binary integrity
- Add `rgaa update` to manually trigger rule update

**`rgaa-holo`:**
- Support multiple LLM providers via configuration (Claude API, OpenAI API)
- Configuration loaded from `~/.config/rgaa/llm.toml`

**`rgaa-storage`:**
- Add SQLite backend for local-only mode (consultants without PostgreSQL)
- PostgreSQL remains for SaaS backend and Team tier

**`rgaa-mcp`:**
- Ensure clean stdio transport for Claude Desktop
- Add `rgaa_source_map` tool for Claude Code integration
- Add `rgaa_verify_fix` tool for agentic re-verification

---

## 3. Install Flow

### 3.1 One-Liner Install

```bash
curl -fsSL https://rgaa.dev/install.sh | sh
```

**Script steps:**

1. **Detect OS/Arch** — download correct binary from GitHub releases
   - `rgaa-x86_64-unknown-linux-gnu`
   - `rgaa-aarch64-unknown-linux-gnu`
   - `rgaa-x86_64-apple-darwin`
   - `rgaa-aarch64-apple-darwin`

2. **Install binary** — `~/.local/bin/rgaa` (or `/usr/local/bin` with sudo)

3. **Detect Chrome/Chromium** — verify CDP-compatible browser is installed
   - Check `google-chrome`, `chromium`, `chromium-browser`
   - If missing: print install instructions for the detected OS

4. **Run `rgaa configure`** — interactive prompts:
   ```
   ? Enter your RGAA SaaS API key: rgaa_sk_...
   ? Select LLM provider:
     > Use my own Claude API key
       Use my own OpenAI API key
       Route through SaaS (included in subscription)
   ? Enter LLM API key: sk-ant-...
   ```

5. **Configure Claude Desktop** — auto-detect and patch config:
   ```json
   // Added to Claude Desktop's claude_desktop_config.json
   {
     "mcpServers": {
       "rgaa": {
         "command": "/home/user/.local/bin/rgaa",
         "args": ["mcp"]
       }
     }
   }
   ```

6. **Initial license validation** — call SaaS API, cache locally

7. **Download latest rules** — fetch rule update feed

8. **Print success:**
   ```
   ✅ RGAA installed successfully!
   
   Binary:   ~/.local/bin/rgaa
   Config:   ~/.config/rgaa/
   Claude:   MCP server configured in Claude Desktop
   
   Try it: Open Claude Desktop and say "Analyse https://example.com pour RGAA"
   ```

### 3.2 File Layout

```
~/.local/bin/rgaa                          # Binary
~/.config/rgaa/
├── license.toml                           # API key, license status, last check-in
├── llm.toml                               # LLM provider config (keys stored here)
├── config.toml                            # General settings (browser path, timeouts)
├── rules/                                 # Cached rule updates from SaaS
│   ├── manifest.json                      # { version, hash, timestamp }
│   ├── axe-rules.json                     # axe-core rule overrides/additions
│   ├── gap-fixes.json                     # JavaScript fix snippets
│   ├── criteria-updates.json              # RGAA criteria changes
│   └── prompts/                           # LLM prompt templates
│       ├── evaluation.txt
│       ├── remediation.txt
│       └── explanation.txt
└── audits/                                # Local audit cache (SQLite)
    └── audits.db
```

### 3.3 Configuration Files

**`~/.config/rgaa/license.toml`:**
```toml
api_key = "rgaa_sk_..."
last_validated = "2026-08-24T10:00:00Z"
grace_days = 7
tier = "professional"
```

**`~/.config/rgaa/llm.toml`:**
```toml
provider = "claude"           # "claude" | "openai" | "saas_proxy"
api_key = "sk-ant-..."        # Only if using own key
model = "claude-sonnet-4-20250514"
max_tokens = 4096
```

**`~/.config/rgaa/config.toml`:**
```toml
[browser]
path = "/usr/bin/google-chrome"
headless = true
timeout_ms = 30000

[analytics]
enabled = true
anonymous_stats = true

[offline]
grace_days = 7
hard_lock_days = 14
```

---

## 4. MCP Tools (Claude Desktop / Claude Code)

### 4.1 Tool Definitions

| Tool | Description | Primary Use |
|------|-------------|-------------|
| `rgaa_analyze` | Full RGAA audit of a URL | "Analyse ce site pour la conformité RGAA" |
| `rgaa_igt` | Isolation Group Testing for a criterion | "Test d'isolation sur le critère 11.1" |
| `rgaa_remediate` | Generate fix code for a violation | "Corrige ce problème de contraste" |
| `rgaa_source_map` | Map DOM element to source file | "Où se trouve ce bouton dans le code?" |
| `rgaa_verify_fix` | Re-check specific violation after fix | "Vérifie que la correction fonctionne" |
| `rgaa_report` | Generate PDF/HTML report | "Génère le rapport d'audit" |
| `rgaa_policy` | RGAA policy for a domain | "Affiche la politique RGAA du site" |
| `rgaa_explain` | Explain criterion in French | "Explique le critère 8.7" |
| `rgaa_evidence` | Capture screenshot/DOM evidence | "Capture une preuve de cette violation" |
| `rgaa_compare` | Compare two audit runs | "Compare les résultats avant/après" |

### 4.2 Claude Desktop Conversation Flow

```
Consultant: "Analyse le site https://client.example.com pour la conformité RGAA"

Claude:
  → rgaa_analyze(url="https://client.example.com")
  → Returns: { summary: { total: 106, passed: 78, failed: 22, critical: 5 },
               violations: [...] }

Consultant: "Montre-moi les violations critiques"

Claude: [filters and presents critical violations with DOM context]

Consultant: "Corrige le problème de contraste sur le bouton principal"

Claude:
  → rgaa_remediate(violation_id="08.03", element="button.submit")
  → Returns: { suggested_fix: "color: #595959", explanation: "..." }
  → [If codebase connected] Applies fix to source file

Consultant: "Génère le rapport PDF"

Claude:
  → rgaa_report(audit_id="...", format="pdf")
  → Returns: report file path
```

### 4.3 Claude Code Agentic Integration

Claude Code has full file system access, enabling the complete audit → fix → verify loop:

**Step 1 — Analyze:**
```
→ rgaa_analyze(url="https://client.example.com")
→ Returns violations with DOM selectors
```

**Step 2 — Map to source:**
```
→ rgaa_source_map(url="...", selector="button.submit")
→ Returns: { source_files: [
    { path: "src/components/ContactForm.tsx", line: 47, confidence: 0.92 },
    { path: "src/styles/forms.css", line: 23, confidence: 0.78 }
  ],
  frameworks_detected: ["react", "tailwind"] }
```

**Step 3 — Get fix:**
```
→ rgaa_remediate(violation_id="08.03", element="button.submit")
→ Returns: { fix_type: "css_update", current: "#666", suggested: "#595959",
             contrast_ratio: { current: 2.8, required: 4.5, suggested: 5.2 } }
```

**Step 4 — Apply fix:**
```
Claude Code edits src/components/ContactForm.tsx:47
```

**Step 5 — Verify:**
```
→ rgaa_verify_fix(url="...", violation_id="08.03")
→ Returns: { fixed: true, new_ratio: 5.2 }
```

**Step 6 — Loop:** Repeat Steps 2-5 for all violations.

**Step 7 — Report:**
```
→ rgaa_report(audit_id="...", format="pdf")
```

### 4.4 Error Recovery in Agentic Loop

When a fix doesn't resolve the violation:

```
→ rgaa_verify_fix(url="...", violation_id="08.03")
→ Returns: { fixed: false, new_ratio: 3.1,
             hint: "Inline style overridden by CSS. Check src/styles/global.css:15" }

Claude Code investigates CSS specificity and applies correct fix.
```

---

## 5. SaaS Backend

### 5.1 Stack

- **Axum** HTTP server (extends existing `rgaa-api`)
- **PostgreSQL** for persistent storage
- **Redis** for rate limiting, license cache
- **Stripe** for billing

### 5.2 API Endpoints

```
Auth & Licensing:
  POST   /api/v1/auth/validate          — Validate API key
  POST   /api/v1/auth/refresh           — Refresh license check-in

Rules:
  GET    /api/v1/rules                  — Fetch latest rules (ETag)
  GET    /api/v1/rules/check            — Check if local rules current

Audits:
  POST   /api/v1/audits                 — Upload audit results
  GET    /api/v1/audits                 — List consultant's audits
  GET    /api/v1/audits/{id}            — Get audit detail
  GET    /api/v1/audits/{id}/report     — Generate PDF report

Usage & Billing:
  GET    /api/v1/usage                  — Usage stats
  GET    /api/v1/billing/subscription   — Current plan
  POST   /api/v1/billing/upgrade        — Upgrade tier
  POST   /api/v1/webhooks/stripe        — Stripe webhook
```

### 5.3 License Model

| Tier | Audits/month | LLM Mode | Storage | Offline Grace |
|------|-------------|----------|---------|---------------|
| Starter | 10 | Own key | 30 days | 7 days |
| Professional | 100 | Own key | 1 year | 7 days |
| Enterprise | Unlimited | SaaS proxy | Unlimited | 30 days |
| Team | Per-seat | Both | Shared pool | 14 days |

### 5.4 Offline Grace Period

- License cached in `~/.config/rgaa/license.toml`
- On startup: attempt validation, update cache
- If offline: use cached `last_validated` timestamp
- **Soft lock** (7+ days): CLI/MCP warns but continues
- **Hard lock** (14+ days): requires re-validation
- Clock manipulation detection: reject if system clock appears rolled back

---

## 6. Remote Rule Updates

### 6.1 Update Feed

The `rgaa-updater` crate polls `GET /api/v1/rules`:

**Update flow:**
1. On startup or every 4 hours: `GET /api/v1/rules/check` with local `manifest.json` hash
2. If `304 Not Modified` → skip
3. If newer version → download, verify signature
4. Atomic swap: download to temp dir, rename into place
5. No binary restart — MCP server picks up new rules on next tool call

### 6.2 Rule Package Structure

```json
{
  "version": "2026.08.24",
  "hash": "sha256:abc123...",
  "signature": "ed25519:...",
  "files": [
    "axe-rules.json",
    "gap-fixes.json",
    "criteria-updates.json",
    "prompts/evaluation.txt",
    "prompts/remediation.txt",
    "prompts/explanation.txt"
  ]
}
```

---

## 7. Security Model

### 7.1 Key Storage

- API key: `~/.config/rgaa/license.toml` with `chmod 600`
- LLM keys: `~/.config/rgaa/llm.toml` with `chmod 600`
- Keys never logged, never sent to MCP client (Claude Desktop)

### 7.2 Transport

- MCP: stdio (no network exposure)
- SaaS API: HTTPS with certificate pinning
- License validation: `Authorization: Bearer` header

### 7.3 Binary Distribution

- GitHub Releases with GPG-signed binaries
- SHA-256 checksums published alongside each release
- Install script verifies checksum before placing binary

### 7.4 SaaS API Security

- Rate limiting per API key (tier-dependent)
- Audit uploads scoped to authenticated consultant
- PostgreSQL row-level security

---

## 8. Analytics & Dashboard

### 8.1 Metrics Collected (Opt-in)

- Audits performed (count, not content)
- Criteria evaluated (which ones, pass/fail rates)
- LLM provider used
- Binary version, OS, architecture
- Rule update freshness

### 8.2 Consultant Dashboard (Web UI)

- Audit history with filters
- Per-audit violation breakdown
- Downloadable PDF/HTML reports
- Usage graphs (audits over time, criteria heatmap)
- Subscription management (Stripe portal)

---

## 9. Implementation Plan

### Phase 1: Foundation (Weeks 1-3)
- [ ] Create `rgaa-license` crate (key management, validation, offline grace)
- [ ] Create `rgaa-updater` crate (rule feed client, atomic updates)
- [ ] Add `rgaa configure` subcommand to CLI
- [ ] Add SQLite backend to `rgaa-storage`
- [ ] Extend `rgaa-holo` for multi-provider LLM support

### Phase 2: MCP Enhancement (Weeks 3-5)
- [ ] Add `rgaa_source_map` tool to MCP server
- [ ] Add `rgaa_verify_fix` tool to MCP server
- [ ] Verify stdio transport works with Claude Desktop
- [ ] Test Claude Code integration end-to-end

### Phase 3: SaaS Backend (Weeks 5-8)
- [ ] Deploy `rgaa-api` as cloud service
- [ ] Implement license validation endpoints
- [ ] Implement audit storage endpoints
- [ ] Implement rule update feed
- [ ] Integrate Stripe billing
- [ ] Build consultant dashboard (web UI)

### Phase 4: Distribution (Weeks 8-10)
- [ ] Write install script (`install.sh`)
- [ ] Set up GitHub Releases with signed binaries
- [ ] Cross-compile for Linux (x86_64, aarch64) and macOS (x86_64, aarch64)
- [ ] Test install flow on clean machines
- [ ] Write documentation

### Phase 5: Launch (Weeks 10-12)
- [ ] Beta testing with 3-5 consultants
- [ ] Pricing page and onboarding flow
- [ ] Support infrastructure
- [ ] Launch

---

## 10. Open Questions

1. **Branding**: What's the public product name? (currently "rgaa-rs" is internal)
2. **Domain**: Where does the SaaS live? (`api.rgaa.dev`, `app.rgaa.dev`?)
3. **Team tier**: How are seats managed? Invite system? SSO?
4. **Custom rules**: Can consultants add their own gap-fix snippets?
5. **Multi-language**: French-only or bilingual (FR/EN) UI and prompts?
