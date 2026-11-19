#!/usr/bin/env node
// A minimal third-party client for the RGAA MCP HTTP transport.
//
// Deliberately dependency-free: `node client.mjs` and nothing else. An
// example that needs `npm install` before it runs is an example nobody
// checks still works.
//
// It performs a real end-to-end call — discover, then invoke — rather than
// asserting a shape from the documentation.

const BASE = process.env.RGAA_MCP_URL ?? "http://127.0.0.1:3000";

let nextId = 1;

/**
 * One JSON-RPC round trip.
 *
 * Two failure classes are distinguished on purpose. A transport failure
 * (connection refused, non-2xx) means the server is not there. A JSON-RPC
 * `error` envelope means the server is there and declined — which for a
 * tool that needs a browser or the network is an ordinary outcome, not a
 * broken integration. Collapsing the two is how a smoke test ends up
 * either useless or permanently red.
 */
// JSON-RPC codes for licence refusals (#87); `error.data` is then a
// `LicenseErrorBody` from @holo-rgaa/mcp-types.
const JSONRPC_LICENSE_REQUIRED = -32002;
const JSONRPC_QUOTA_EXCEEDED = -32003;

// Longest `Retry-After` this example will sit through before retrying once.
// A daily quota resets at 00:00 UTC — hours away — so it is reported, never
// waited out.
const MAX_RETRY_WAIT_SECS = 30;

/**
 * A licence refusal (#87), from an HTTP 401/402/429 body or a JSON-RPC
 * -32002/-32003 `error.data`. Both carry the same object; discriminate on
 * `code`, never on the message.
 */
function licenseError(method, body, retryAfterHeader) {
  const retryAfter = Number(body?.retry_after ?? retryAfterHeader ?? NaN);
  let message;
  switch (body?.code) {
    case "feature_not_in_tier":
      // Waiting does not change a tier: no retry.
      message =
        `${method}: needs the ${body.required_tier} tier (current: ${body.tier}).` +
        (body.upgrade_url ? ` Upgrade: ${body.upgrade_url}` : "") +
        " Then on the server host: rgaa licence activate <key>";
      break;
    case "license_expired":
      message = `${method}: the server's licence expired or its offline grace ran out (now ${body.tier}).`;
      break;
    case "license_invalid":
      message = `${method}: the server's licence is invalid — check it with \`rgaa licence status\` on the server host.`;
      break;
    case "quota_exceeded":
      message =
        `${method}: daily quota reached (${body.limit - body.remaining}/${body.limit}).` +
        (Number.isFinite(retryAfter)
          ? ` Resets in ${Math.ceil(retryAfter / 60)} min (${body.reset_at}).`
          : "");
      break;
    default:
      message = `${method}: refused by the licence gate (${body?.error ?? "no details"})`;
  }
  const err = new Error(message);
  err.code = body?.code ?? "license";
  err.license = body ?? null;
  err.retryAfter = Number.isFinite(retryAfter) ? retryAfter : null;
  return err;
}

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

async function rpc(method, params, { retried = false } = {}) {
  const response = await fetch(`${BASE}/mcp`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: nextId++, method, params }),
  });

  if (response.status === 204) return null; // notifications

  // Licence refusals on the HTTP layer. 401 is also the bearer-token check
  // (`--auth-token`), which has no licence body, so only a body with a
  // licence `code` is treated as one.
  if ([401, 402, 429].includes(response.status)) {
    const body = await response.json().catch(() => null);
    const retryAfterHeader = response.headers.get("retry-after");
    if (body?.code) {
      const err = licenseError(method, body, retryAfterHeader);
      // A daily quota is reported once, never retried in a loop.
      throw err;
    }
    // A generic 429 with a short Retry-After: honour it, retry once.
    const wait = Number(retryAfterHeader);
    if (response.status === 429 && !retried && wait >= 0 && wait <= MAX_RETRY_WAIT_SECS) {
      console.log(`  rate limited, retrying once in ${wait}s`);
      await sleep(wait * 1000);
      return rpc(method, params, { retried: true });
    }
  }
  if (!response.ok) {
    throw new Error(`transport: HTTP ${response.status} from ${BASE}/mcp`);
  }

  const body = await response.json();
  if (body.error) {
    if (
      body.error.code === JSONRPC_LICENSE_REQUIRED ||
      body.error.code === JSONRPC_QUOTA_EXCEEDED
    ) {
      throw licenseError(method, body.error.data);
    }
    const err = new Error(`${method}: ${body.error.message}`);
    err.rpc = body.error;
    // The stable code lives in `data.code`. Matching on message text breaks
    // the moment a message is reworded.
    err.code = body.error.data?.code ?? null;
    throw err;
  }
  return body.result;
}

async function main() {
  console.log(`→ ${BASE}`);

  const init = await rpc("initialize", {});
  console.log(`  server: ${init.serverInfo.name} ${init.serverInfo.version}`);
  console.log(`  protocol: ${init.protocolVersion}`);

  // Discover rather than hardcode. The tool set grows; a client that
  // assumes six tools goes stale the way the plugin docs did.
  const { tools } = await rpc("tools/list", {});
  console.log(`  tools (${tools.length}): ${tools.map((t) => t.name).join(", ")}`);

  // `list_criteria` is the one tool that needs neither a browser nor the
  // network, so it proves the path end to end without external dependencies.
  const result = await rpc("tools/call", {
    name: "list_criteria",
    arguments: {},
  });

  const criteria = result.structuredContent?.criteria;
  if (!Array.isArray(criteria)) {
    throw new Error("list_criteria returned no structuredContent.criteria");
  }
  if (criteria.length !== 106) {
    throw new Error(`expected 106 RGAA criteria, got ${criteria.length}`);
  }

  const [first] = criteria;
  console.log(`  list_criteria: ${criteria.length} criteria`);
  console.log(`    e.g. ${first.id} — ${first.title} (${first.classification})`);
  console.log("\n✓ end-to-end tool call succeeded");
}

main().catch((error) => {
  console.error(`\n✗ ${error.message}`);
  if (error.code) console.error(`  code: ${error.code}`);
  if (error.cause) console.error(`  cause: ${error.cause}`);
  if (error.license) {
    // The server answered: it is running, so the hint below would mislead.
    process.exit(2);
  }
  console.error(
    `\nIs the server running?\n  rgaa-mcp-http --host 127.0.0.1 --port 3000`,
  );
  process.exit(1);
});
