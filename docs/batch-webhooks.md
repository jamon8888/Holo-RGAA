# Batch webhooks: signature verification

When a batch finishes, the server POSTs the result to the callback URL you
registered. Because that endpoint accepts unauthenticated inbound requests
by construction, **anyone who learns the URL can forge a completion**. The
signature is what makes the callback evidence rather than a rumour.

## The header

```
x-rgaa-signature: t=1700000000,v1=3f9a…c2
```

| Part | Meaning |
|---|---|
| `t` | Unix seconds when the delivery was signed |
| `v1` | Lowercase hex HMAC-SHA256 |

## What is signed

```
HMAC_SHA256(secret, "<t>" + "." + <raw request body bytes>)
```

Two details are load-bearing.

**The `.` separator.** Signing `t ‖ body` with no delimiter makes
`("12", "3{…}")` and `("123", "{…}")` hash identical bytes, so a signature
issued for one timestamp is valid for another — which defeats the
timestamp check below. A byte that cannot occur in a decimal timestamp
removes the ambiguity.

**Raw bytes, not a re-serialization.** Hash exactly what arrived on the
wire. If you parse the JSON and re-serialize it before hashing, key order,
float formatting and escaping will differ from what the sender hashed and
every delivery will fail. Read the body as bytes *before* your framework's
JSON parser touches it.

## Verifying

1. Read `x-rgaa-signature`. Missing → reject.
2. Parse `t` and `v1`. Malformed → reject.
3. `abs(now - t) > 300` → reject as replay. Check the **absolute**
   difference: a timestamp far in the future is as suspect as a stale one,
   and `now - t` alone underflows on it.
4. Recompute the HMAC over `"<t>.<raw body>"`.
5. Compare in **constant time**. A byte-by-byte `==` that returns early
   leaks the expected digest one byte at a time through response timing.

Step 3 is not optional. A correctly signed body stays correctly signed
forever, so without a bound on `t` a captured delivery can be replayed at
will. The ±300s window is the usual allowance for clock skew between two
machines not running NTP against each other.

## Node

```js
import crypto from "node:crypto";

// express: app.post(path, express.raw({ type: "application/json" }), handler)
// so `req.body` is a Buffer, not a parsed object.
export function verify(secret, header, rawBody, now = Math.floor(Date.now() / 1000)) {
  if (!header) return false;

  const parts = Object.fromEntries(
    header.split(",").map((p) => p.trim().split("=", 2)),
  );
  const t = Number(parts.t);
  if (!Number.isInteger(t) || !parts.v1) return false;

  if (Math.abs(now - t) > 300) return false;

  const expected = crypto
    .createHmac("sha256", secret)
    .update(`${t}.`)
    .update(rawBody)
    .digest();

  const provided = Buffer.from(parts.v1, "hex");
  // timingSafeEqual throws on a length mismatch, so check first.
  if (provided.length !== expected.length) return false;
  return crypto.timingSafeEqual(provided, expected);
}
```

## Python

```python
import hashlib, hmac, time

def verify(secret: bytes, header: str | None, raw_body: bytes, now: int | None = None) -> bool:
    if not header:
        return False
    now = int(time.time()) if now is None else now

    parts = dict(
        p.strip().split("=", 1) for p in header.split(",") if "=" in p
    )
    try:
        t = int(parts["t"])
        provided = bytes.fromhex(parts["v1"])
    except (KeyError, ValueError):
        return False

    if abs(now - t) > 300:
        return False

    expected = hmac.new(
        secret, f"{t}.".encode() + raw_body, hashlib.sha256
    ).digest()
    return hmac.compare_digest(provided, expected)
```

## Rust

The server's own implementation is `rgaa_api::webhook`, and
`webhook::verify` is the receiver-side function — usable directly if your
receiver is also Rust.

## Rotating the secret

Accept either of two secrets during a rollover and retire the old one once
no delivery has been signed with it for longer than the 300s window.
