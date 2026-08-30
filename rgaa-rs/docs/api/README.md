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

## Base URL

```
http://localhost:3000
```

## Endpoints

---

### `POST /audit`

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

The API has CORS enabled for all origins, methods, and headers.

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
