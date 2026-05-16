/**
 * Spawn `agentspay-mcp`, run a single tool call through MCP 2025-06-18 stdio,
 * return the inner result JSON.
 *
 * We send three messages in order: `initialize`, `notifications/initialized`,
 * `tools/call`. Then we read line-delimited JSON from stdout until we see the
 * response for our tools/call id. We deliberately keep stdin OPEN until that
 * response arrives — the rmcp server treats stdin EOF as a shutdown signal
 * and would exit before our tool handler finishes. After the response we
 * close stdin and kill+reap the child.
 *
 * One subprocess per call: simple, reliable, matches what services/web-shim
 * does in Rust. Cold-start cost is ~100-150ms; that's dwarfed by the on-chain
 * settlement time of pay_url and acceptable for the read-only tools.
 */

import { spawn, type ChildProcess } from "node:child_process";
import { existsSync } from "node:fs";
import { delimiter as PATH_DELIMITER, join } from "node:path";

import {
  BinaryNotFoundError,
  TransportError,
  TransportTimeoutError,
  classifyToolError,
} from "./errors.js";
import type { AgentsPayClientOptions, JsonValue, Network } from "./types.js";

const TOOL_CALL_ID = 42;
const INITIALIZE_ID = 1;
const DEFAULT_NETWORK: Network = "solana-devnet";
const DEFAULT_TIMEOUT_MS = 30_000;
const PROTOCOL_VERSION = "2025-06-18";
const SDK_VERSION = "0.2.0";

interface JsonRpcResponse {
  readonly jsonrpc: "2.0";
  readonly id?: number;
  readonly result?: JsonValue;
  readonly error?: {
    readonly code: number;
    readonly message: string;
    readonly data?: JsonValue;
  };
}

export interface ResolvedTransportConfig {
  readonly binPath: string;
  readonly network: Network;
  readonly callTimeoutMs: number;
  readonly env: Readonly<Record<string, string>>;
  readonly debug: boolean;
}

/**
 * Resolve the SDK options into a concrete transport config. Locates the
 * binary; throws BinaryNotFoundError if it can't be found.
 */
export function resolveTransport(
  options: AgentsPayClientOptions = {},
): ResolvedTransportConfig {
  const network = options.network ?? DEFAULT_NETWORK;
  const callTimeoutMs = options.callTimeoutMs ?? DEFAULT_TIMEOUT_MS;
  const debug = options.debug ?? false;

  const env: Record<string, string> = {
    AGENTSPAY_NETWORK: network,
    RUST_LOG: "agentspay_mcp=warn",
    ...(options.env ?? {}),
  };
  if (options.keypairPath !== undefined) {
    env.AGENTSPAY_KEYPAIR_PATH = options.keypairPath;
  }
  if (options.databaseUrl !== undefined) {
    env.AGENTSPAY_DATABASE_URL = options.databaseUrl;
  }

  const binPath = locateBinary(options.mcpBinPath);
  return { binPath, network, callTimeoutMs, env, debug };
}

function locateBinary(explicit?: string): string {
  const searched: string[] = [];

  if (explicit !== undefined) {
    searched.push(explicit);
    if (existsSync(explicit)) return explicit;
  }

  const envPath = process.env.AGENTSPAY_MCP_BIN;
  if (envPath !== undefined && envPath !== "") {
    searched.push(envPath);
    if (existsSync(envPath)) return envPath;
  }

  const pathEntries = (process.env.PATH ?? "").split(PATH_DELIMITER);
  for (const dir of pathEntries) {
    if (dir === "") continue;
    const candidate = join(dir, "agentspay-mcp");
    if (existsSync(candidate)) return candidate;
  }

  searched.push("$PATH/agentspay-mcp", "$AGENTSPAY_MCP_BIN");
  throw new BinaryNotFoundError(searched);
}

/**
 * Run one MCP tool call. Returns the parsed payload from the JSON-RPC
 * response, or throws a typed error.
 */
export async function callTool(
  config: ResolvedTransportConfig,
  tool: string,
  args: JsonValue,
): Promise<JsonValue> {
  const child = spawn(config.binPath, [], {
    stdio: ["pipe", "pipe", config.debug ? "inherit" : "ignore"],
    env: { ...process.env, ...config.env },
  });

  let stdoutBuffer = "";
  let resolved = false;

  const resultPromise = new Promise<JsonValue>((resolve, reject) => {
    const onTimeout = setTimeout(() => {
      if (resolved) return;
      resolved = true;
      cleanup(child);
      reject(new TransportTimeoutError(config.callTimeoutMs, tool));
    }, config.callTimeoutMs);

    child.on("error", (err) => {
      if (resolved) return;
      resolved = true;
      clearTimeout(onTimeout);
      cleanup(child);
      reject(
        new TransportError(
          `failed to spawn ${config.binPath}: ${err.message}`,
          { cause: err },
        ),
      );
    });

    child.on("exit", (code, signal) => {
      if (resolved) return;
      resolved = true;
      clearTimeout(onTimeout);
      reject(
        new TransportError(
          `agentspay-mcp exited unexpectedly (code=${code}, signal=${signal}) before tool '${tool}' completed`,
        ),
      );
    });

    child.stdout.on("data", (chunk: Buffer) => {
      stdoutBuffer += chunk.toString("utf8");
      const lines = stdoutBuffer.split("\n");
      stdoutBuffer = lines.pop() ?? "";
      for (const rawLine of lines) {
        const line = rawLine.trim();
        if (line === "" || resolved) continue;

        let msg: JsonRpcResponse;
        try {
          msg = JSON.parse(line) as JsonRpcResponse;
        } catch (parseErr) {
          resolved = true;
          clearTimeout(onTimeout);
          cleanup(child);
          reject(
            new TransportError(
              `malformed JSON-RPC line from agentspay-mcp: ${line}`,
              { cause: parseErr },
            ),
          );
          return;
        }

        if (msg.id !== TOOL_CALL_ID) continue;

        resolved = true;
        clearTimeout(onTimeout);

        if (msg.error !== undefined) {
          cleanup(child);
          reject(classifyToolError(tool, msg.error.message, msg.error.data));
          return;
        }

        const unwrapped = unwrapToolResult(tool, msg.result ?? null);
        cleanup(child);
        if (unwrapped instanceof Error) {
          reject(unwrapped);
        } else {
          resolve(unwrapped);
        }
      }
    });
  });

  const initialize = JSON.stringify({
    jsonrpc: "2.0",
    id: INITIALIZE_ID,
    method: "initialize",
    params: {
      protocolVersion: PROTOCOL_VERSION,
      capabilities: {},
      clientInfo: { name: "@agentspay/sdk-js", version: SDK_VERSION },
    },
  });
  const initialized = JSON.stringify({
    jsonrpc: "2.0",
    method: "notifications/initialized",
    params: {},
  });
  const toolCall = JSON.stringify({
    jsonrpc: "2.0",
    id: TOOL_CALL_ID,
    method: "tools/call",
    params: { name: tool, arguments: args },
  });

  child.stdin.write(`${initialize}\n${initialized}\n${toolCall}\n`);

  return resultPromise;
}

/**
 * MCP tools return CallToolResult shapes. Unwrap to the actual payload, or
 * convert an `isError: true` result into a typed ToolError.
 */
function unwrapToolResult(tool: string, raw: JsonValue): JsonValue | Error {
  if (raw === null || typeof raw !== "object" || Array.isArray(raw)) {
    return new TransportError(
      `expected CallToolResult object from tool '${tool}', got ${typeof raw}`,
    );
  }
  const result = raw as { readonly [key: string]: JsonValue };

  if (result.structuredContent !== undefined && result.structuredContent !== null) {
    if (result.isError === true) {
      return classifyToolError(tool, stringifyError(result.structuredContent));
    }
    return result.structuredContent;
  }

  const content = result.content;
  if (!Array.isArray(content) || content.length === 0) {
    if (result.isError === true) {
      return classifyToolError(tool, "tool returned an error with no content");
    }
    return null;
  }
  const first = content[0];
  if (first === null || typeof first !== "object" || Array.isArray(first)) {
    return new TransportError(`unexpected content shape from tool '${tool}'`);
  }
  const text = (first as { readonly text?: JsonValue }).text;
  if (typeof text !== "string") {
    if (result.isError === true) {
      return classifyToolError(tool, "tool returned an error with no text content");
    }
    return null;
  }

  let parsed: JsonValue;
  try {
    parsed = JSON.parse(text) as JsonValue;
  } catch {
    parsed = text;
  }

  if (result.isError === true) {
    return classifyToolError(tool, stringifyError(parsed));
  }
  return parsed;
}

function stringifyError(value: JsonValue): string {
  if (typeof value === "string") return value;
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}

function cleanup(child: ChildProcess): void {
  try {
    child.stdin?.end();
  } catch {
    // already closed
  }
  if (!child.killed) {
    child.kill("SIGTERM");
  }
}
