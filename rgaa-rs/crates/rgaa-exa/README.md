# rgaa-exa

Exa web-search grounding for RGAA audits.

RGAA evaluation is grounded on a frozen regulatory corpus (`rgaa-agent`'s
LanceDB référentiel). This crate covers what that corpus cannot: **current
external guidance** — WCAG techniques, ARIA authoring practices, vendor
accessibility notes — retrieved at audit time so a remediation proposal cites
a checkable source instead of model recall.

## Setup

```bash
export EXA_API_KEY="your_api_key_here"
```

Workspace binaries load the repo-root `.env` through `dotenvy`, so the key can
live there instead (`.env` is gitignored). Optional: `EXA_BASE_URL`,
`EXA_TIMEOUT_SECS`.

## Usage

```rust,no_run
use rgaa_core::RgaaCriteria;
use rgaa_exa::{guidance, remediation_guidance, ExaClient};

# async fn run() -> Result<(), Box<dyn std::error::Error>> {
let client = ExaClient::from_env()?;
let criterion = RgaaCriteria::all().iter().find(|c| c.id == "1.3").unwrap();

let refs = remediation_guidance(&client, criterion, Some("img sans attribut alt")).await?;
print!("{}", guidance::render(&refs)); // ready to append to a prompt or report
# Ok(())
# }
```

```bash
cargo run -p rgaa-exa --example criterion_guidance -- 1.3 "img sans attribut alt"
```

## Endpoint choice

`POST /search` with `type: "auto"` and `contents: { "highlights": true }` —
Exa's recommended request, and nothing more:

- **`/search`, not `/answer`** — the workspace already has an LLM
  (`rgaa-holo`). Exa returns raw grounded excerpts; synthesis stays in
  `rgaa-agent`, where the RGAA verdict schema and the verifier live.
- **`/search`, not `/agent`** — per-criterion lookup is a single-shot
  retrieval on a latency budget, not async multi-step list-building.
- **`highlights`, not `text` or `summary`** — highlights are token-efficient
  and fit the byte-capped "Références" prompt section. `summary` would fire a
  per-result LLM call and duplicate work `rgaa-agent` already does.
- **No `category`, no `includeDomains`, no date filters.** Source preferences
  ("documentation officielle", "à jour") and the WCAG success criteria are
  phrased into the query instead; hard filters would silently drop good pages.
- **`numResults: 5`** is the one deliberate departure from the server default
  of 10: the prompt's references section is byte-capped, so the extra results
  would only be paid for and then truncated.

Data retention: `/search` is a Zero Data Retention surface, which matters for
audits of client sites.
