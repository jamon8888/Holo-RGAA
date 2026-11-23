# RGAA API Reference

The `rgaa-api` binary provides a RESTful HTTP API for running RGAA accessibility audits, managing audit results, and retrieving criteria information.

## Installation

```bash
cargo install --path crates/rgaa-api
```

Or use the pre-built binary from the [release page](https://github.com/jamon8888/Holo-RGAA/releases).

## Running the Server

```bash
# Start with default settings (listens on 0.0.0.0:3000)
rgaa-api

# Custom listen address
LISTEN_ADDR=0.0.0.0:8080 rgaa-api

# PostgreSQL connection
DATABASE_URL=postgres://user:pass@localhost/rgaa rgaa-api
```

**Environment Variables:**

| Variable | Description | Default |
|----------|-------------|---------|
| `LISTEN_ADDR` | Address to bind to | `0.0.0.0:3000` |
| `DATABASE_URL` | PostgreSQL connection string | `postgres://localhost/rgaa` |
| `RGAA_LICENSE_KEY` | Licence token (overrides `~/.config/rgaa/license.toml`) | none: Free tier |
| `RGAA_LICENSE_PUBLIC_KEYS` | Extra `k4.public.…` keys trusted to sign licences | none |
| `RGAA_LICENSE_SERVER` | Non-empty toggle for the offline grace period; its value is not read and no revalidation request is made | unset (grace disabled) |
| `RGAA_LICENSE_GRACE_DAYS` | Offline grace period, in days | `7` |

## Licensing (#87)

The server's licence decides which routes it serves and how many metered
units it accepts per UTC day (Free 10, Professional 500, Team 5000). Clients
send nothing extra; activate the licence on the server host with
`rgaa licence activate <key>` or set `RGAA_LICENSE_KEY`.

**Response headers** on every licence-checked route (exposed through CORS):

| Header | Value |
|--------|-------|
| `X-License-Tier` | `free`, `professional`, `team` |
| `X-License-Status` | `unlicensed`, `valid`, `grace`, `expired`, `grace_exceeded`, `invalid` |
| `X-License-Grace-Remaining` | Seconds of offline grace left (status `grace` only) |
| `X-RateLimit-Limit` / `X-RateLimit-Remaining` | Daily units |
| `X-RateLimit-Reset` | Unix seconds of the next reset (00:00 UTC) |
| `Retry-After` | Seconds to wait (429 only) |

**Refusals:**

| Status | `code` | Meaning | Retry? |
|--------|--------|---------|--------|
| `401` | `license_invalid` | Licence present but unusable (signature, format, not yet valid) | No: fix the licence |
| `402` | `feature_not_in_tier` | Route needs a higher tier (`required_tier`) | No: upgrade (`upgrade_url`) |
| `402` | `license_expired` | Licence expired, or offline grace exceeded | Renew the licence |
| `429` | `quota_exceeded` | Daily quota used up | After `Retry-After` |

```json
{
  "error": "daily quota of 500 reached; resets at 2026-11-20T00:00:00Z",
  "code": "quota_exceeded",
  "tier": "professional",
  "feature": "full_audit",
  "limit": 500,
  "remaining": 0,
  "reset_at": "2026-11-20T00:00:00Z",
  "retry_after": 11520
}
```

The body extends the existing `{"error": "..."}` shape, so clients that read
only `error` keep working. TypeScript: `LicenseErrorBody` in
`@holo-rgaa/mcp-types`.

The legacy `POST /audit` and `GET /audit/{id}` routes are **not**
licence-checked (nor authenticated) and are **deprecated**; use `/v1/*`.

## Base URL

```
http://localhost:3000
```

## Endpoints

---

### `POST /audit`

> **Deprecated.** Not authenticated and not licence-checked; use `/v1/*`.

Run a new RGAA accessibility audit.

**Request:**

```json
{
  "url": "https://example.test"
}
```

**Response:** `200 OK`

```json
{
  "audit_id": "aud_abc123xyz",
  "url": "https://example.test",
  "taux_global": 85.5,
  "coverage_percent": 92.3,
  "etat_conformite": "partielle",
  "passed": 45,
  "failed": 8,
  "na": 53
}
```

---

### `GET /audit/{id}`

> **Deprecated.** Not licence-checked; use `/v1/*`.

Retrieve a previously run audit by its ID.

**Response:** `200 OK`

```json
{
  "audit_id": "aud_abc123xyz",
  "url": "https://example.test",
  "taux_global": 85.5,
  "coverage_percent": 92.3,
  "etat_conformite": "partielle",
  "passed": 45,
  "failed": 8,
  "na": 53
}
```

**Error Responses:**

| Status | Description |
|--------|-------------|
| `404 Not Found` | Audit not found |

---

### `GET /criteria`

List all 106 RGAA criteria with their IDs, titles, and classifications.

**Response:** `200 OK`

```json
[
  {
    "id": "1.1",
    "title": "Each image has an alternative",
    "classification": "Deterministe"
  },
  {
    "id": "1.3",
    "title": "Complex images have a detailed description",
    "classification": "IaAssiste"
  }
]
```

**Classification Values:**

| Value | Description |
|-------|-------------|
| `Deterministe` | Automatically testable (axe-core + gap-fix) |
| `IaAssiste` | Requires LLM-assisted evaluation |
| `Manuel` | Manual testing required |

---

### `GET /health`

Health check endpoint.

**Response:** `200 OK`

```
OK
```

---

## CORS

The API has CORS enabled for all origins, methods, and headers. The licence
and rate-limit headers above are listed in `Access-Control-Expose-Headers`.

## Example Usage

### cURL

```bash
# Run an audit
curl -X POST http://localhost:3000/audit \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.test"}'

# Get audit result
curl http://localhost:3000/audit/aud_abc123xyz

# List criteria
curl http://localhost:3000/criteria
```

### JavaScript (Fetch)

```javascript
// Run an audit
const auditResponse = await fetch('http://localhost:3000/audit', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ url: 'https://example.test' })
});
const audit = await auditResponse.json();
```

### Python

```python
import requests

response = requests.post('http://localhost:3000/audit', json={
    'url': 'https://example.test'
})
audit = response.json()
```

## Deployment

### Docker

```dockerfile
FROM ghcr.io/jamon8888/rgaa-api:latest
ENV DATABASE_URL=postgres://user:pass@db:5432/rgaa
EXPOSE 3000
CMD ["rgaa-api"]
```

### Docker Compose

```yaml
services:
  api:
    image: ghcr.io/jamon8888/rgaa-api:latest
    ports:
      - "3000:3000"
    environment:
      - DATABASE_URL=postgres://user:pass@db:5432/rgaa
    depends_on:
      - db

  db:
    image: postgres:16
    environment:
      - POSTGRES_DB=rgaa
      - POSTGRES_USER=user
      - POSTGRES_PASSWORD=pass
```

### Production Deployment

1. Use a reverse proxy (nginx, Caddy) with TLS termination
2. Configure proper CORS origins
3. Set up PostgreSQL with connection pooling (PgBouncer)
4. Add monitoring and authentication
