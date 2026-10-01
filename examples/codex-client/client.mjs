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
async function rpc(method, params) {
  const response = await fetch(`${BASE}/mcp`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ jsonrpc: "2.0", id: nextId++, method, params }),
  });

  if (response.status === 204) return null; // notifications
  if (!response.ok) {
    throw new Error(`transport: HTTP ${response.status} from ${BASE}/mcp`);
  }

  const body = await response.json();
  if (body.error) {
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
  console.error(
    `\nIs the server running?\n  rgaa-mcp-http --host 127.0.0.1 --port 3000`,
  );
  process.exit(1);
});
