import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const runtimePath = (() => {
  if (process.env.SNAPMCP_RUNTIME) return process.env.SNAPMCP_RUNTIME;
  if (process.platform === "win32") {
    return path.join(process.env.APPDATA || path.join(os.homedir(), "AppData", "Roaming"), "SnapMCP", "runtime.json");
  }
  if (process.platform === "darwin") {
    return path.join(os.homedir(), "Library", "Application Support", "SnapMCP", "runtime.json");
  }
  return path.join(process.env.XDG_CONFIG_HOME || path.join(os.homedir(), ".config"), "snapmcp", "runtime.json");
})();

let runtime = {};
try {
  runtime = JSON.parse(fs.readFileSync(runtimePath, "utf8"));
} catch {
  // The app may still be running on the default port; try that below.
}

const endpoint =
  process.env.SNAPMCP_ENDPOINT || runtime.endpoint || "http://127.0.0.1:37621/mcp";
const token = process.env.SNAPMCP_TOKEN || runtime.token || "";

async function rpc(id, method, params = {}) {
  const headers = {
    "Content-Type": "application/json",
    Accept: "application/json, text/event-stream",
  };
  if (token) headers.Authorization = `Bearer ${token}`;
  const res = await fetch(endpoint, {
    method: "POST",
    headers,
    body: JSON.stringify({ jsonrpc: "2.0", id, method, params }),
  });
  const text = await res.text();
  let json = null;
  try {
    json = JSON.parse(text);
  } catch {}
  return { status: res.status, json, text };
}

function fail(message) {
  console.error(`✗ ${message}`);
  process.exitCode = 1;
}

console.log(`SnapMCP endpoint: ${endpoint}`);
if (!runtime.port) console.log(`(runtime.json not found at ${runtimePath})`);

const init = await rpc(1, "initialize", {
  protocolVersion: "2025-11-25",
  capabilities: {},
  clientInfo: { name: "snapmcp-smoke", version: "1.0.0" },
});
if (init.json?.result?.serverInfo) {
  console.log(
    `✓ initialize: ${init.json.result.serverInfo.name} v${init.json.result.serverInfo.version} (${init.json.result.protocolVersion})`,
  );
} else {
  fail(`initialize failed: HTTP ${init.status} ${init.text.slice(0, 300)}`);
}

const tools = await rpc(2, "tools/list");
if (Array.isArray(tools.json?.result?.tools)) {
  console.log(`✓ tools/list: ${tools.json.result.tools.map((t) => t.name).join(", ")}`);
} else {
  fail(`tools/list failed: HTTP ${tools.status} ${tools.text.slice(0, 300)}`);
}

const displays = await rpc(3, "tools/call", {
  name: "list_displays",
  arguments: {},
});
const displayText = displays.json?.result?.content?.find((c) => c.type === "text")?.text;
if (displayText) {
  console.log("✓ list_displays:");
  console.log(displayText);
} else {
  fail(`list_displays failed: HTTP ${displays.status} ${displays.text.slice(0, 300)}`);
}

if (process.argv.includes("--capture")) {
  const shot = await rpc(4, "tools/call", {
    name: "take_screenshot",
    arguments: { display: "primary", includeImage: false },
  });
  const text = shot.json?.result?.content?.find((c) => c.type === "text")?.text;
  if (text) {
    console.log("✓ take_screenshot:");
    console.log(text);
  } else {
    fail(`take_screenshot failed: HTTP ${shot.status} ${shot.text.slice(0, 300)}`);
  }
}

const discover = await rpc(5, "server/discover", {
  _meta: {
    "io.modelcontextprotocol/protocolVersion": "2026-07-28",
    "io.modelcontextprotocol/clientInfo": { name: "snapmcp-smoke", version: "1.0.0" },
    "io.modelcontextprotocol/clientCapabilities": {},
  },
});
if (Array.isArray(discover.json?.result?.supportedVersions)) {
  console.log(`✓ server/discover: ${discover.json.result.supportedVersions.join(", ")}`);
} else {
  console.log("· server/discover unavailable (legacy-only server?)");
}

if (!process.exitCode) console.log("\nMCP smoke test passed.");
