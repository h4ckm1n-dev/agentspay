#!/usr/bin/env node

type AgentsPayEnvironment = "sandbox" | "live";

interface CliOptions {
  baseUrl: string;
  apiKey: string | undefined;
  environment: AgentsPayEnvironment;
  json: boolean;
  debug: boolean;
}

interface ParsedArgs {
  command: readonly string[];
  options: CliOptions;
}

class CliError extends Error {
  readonly status: number | undefined;
  readonly details: unknown | undefined;

  constructor(message: string, options: { readonly status?: number; readonly details?: unknown } = {}) {
    super(message);
    this.name = "CliError";
    this.status = options.status;
    this.details = options.details;
  }
}

const DEFAULT_BASE_URL = "http://localhost:8080";

void main(process.argv.slice(2)).catch((error: unknown) => {
  if (error instanceof CliError) {
    console.error(error.message);
    if (error.status !== undefined) {
      console.error(`status: ${error.status}`);
    }
    if (error.details !== undefined) {
      console.error(formatValue(error.details));
    }
  } else if (error instanceof Error) {
    console.error(error.message);
  } else {
    console.error(String(error));
  }
  process.exitCode = 1;
});

async function main(argv: readonly string[]): Promise<void> {
  const parsed = parseArgs(argv);
  const [command, subcommand] = parsed.command;

  if (command === undefined || command === "help" || command === "--help" || command === "-h") {
    printHelp();
    return;
  }

  if (command === "status") {
    await handleStatus(parsed.options);
    return;
  }

  if (command === "balance") {
    await handleBalance(parsed.options);
    return;
  }

  if (command === "endpoints" && subcommand === "list") {
    await handleEndpointsList(parsed.options);
    return;
  }

  if (command === "demo") {
    handleDemo(parsed.options);
    return;
  }

  throw new CliError(`Unknown command: ${parsed.command.join(" ")}`);
}

function parseArgs(argv: readonly string[]): ParsedArgs {
  const options: CliOptions = {
    baseUrl: process.env.AGENTSPAY_BASE_URL ?? DEFAULT_BASE_URL,
    apiKey: process.env.AGENTSPAY_API_KEY,
    environment: "sandbox",
    json: false,
    debug: false,
  };
  const command: string[] = [];

  for (let index = 0; index < argv.length; index += 1) {
    const arg = argv[index];

    if (arg === "--base-url") {
      options.baseUrl = requireValue(argv, index, "--base-url");
      index += 1;
      continue;
    }

    if (arg === "--api-key") {
      options.apiKey = requireValue(argv, index, "--api-key");
      index += 1;
      continue;
    }

    if (arg === "--environment") {
      options.environment = parseEnvironment(requireValue(argv, index, "--environment"));
      index += 1;
      continue;
    }

    if (arg === "--sandbox") {
      options.environment = "sandbox";
      continue;
    }

    if (arg === "--live") {
      options.environment = "live";
      continue;
    }

    if (arg === "--json") {
      options.json = true;
      continue;
    }

    if (arg === "--debug") {
      options.debug = true;
      continue;
    }

    command.push(arg ?? "");
  }

  options.baseUrl = options.baseUrl.replace(/\/+$/, "");
  return { command, options };
}

function requireValue(argv: readonly string[], index: number, flag: string): string {
  const value = argv[index + 1];
  if (value === undefined || value.startsWith("--")) {
    throw new CliError(`Missing value for ${flag}`);
  }
  return value;
}

function parseEnvironment(value: string): AgentsPayEnvironment {
  if (value === "sandbox" || value === "live") {
    return value;
  }
  throw new CliError(`Invalid environment: ${value}`);
}

async function handleStatus(options: CliOptions): Promise<void> {
  const payload = await apiGet(options, "/v1/status");
  render("AgentsPay status", payload, options);
}

async function handleBalance(options: CliOptions): Promise<void> {
  const payload = await apiGet(options, "/v1/balances");
  render("AgentsPay balance", payload, options);
}

async function handleEndpointsList(options: CliOptions): Promise<void> {
  const payload = await apiGet(options, "/v1/endpoints");
  render("AgentsPay endpoints", payload, options);
}

function handleDemo(options: CliOptions): void {
  const steps = [
    "Initialize sandbox client defaults.",
    "Request a protected endpoint.",
    "Handle HTTP 402 Payment Required.",
    "Authorize the returned payment requirement.",
    "Retry with PAYMENT-SIGNATURE and PAYMENT-RESPONSE headers.",
    "Verify, settle, and record an audit proof.",
  ];

  if (options.json) {
    console.log(
      JSON.stringify(
        {
          command: "demo",
          mode: options.environment,
          base_url: options.baseUrl,
          steps,
          status: "scaffold",
        },
        null,
        2,
      ),
    );
    return;
  }

  console.log("AgentsPay demo");
  console.log(`mode: ${options.environment}`);
  console.log(`api: ${options.baseUrl}`);
  for (const [index, step] of steps.entries()) {
    console.log(`${index + 1}. ${step}`);
  }
}

async function apiGet(options: CliOptions, path: string): Promise<unknown> {
  const headers = new Headers({
    Accept: "application/json",
    "X-AgentsPay-Environment": options.environment,
  });

  if (options.apiKey !== undefined && options.apiKey.length > 0) {
    headers.set("Authorization", `Bearer ${options.apiKey}`);
  }

  const url = `${options.baseUrl}${path}`;
  if (options.debug) {
    console.error(`[agentspay] GET ${url}`);
  }

  const response = await fetch(url, {
    method: "GET",
    headers,
  });
  const payload = await readPayload(response);

  if (!response.ok) {
    throw new CliError(`AgentsPay request failed for ${path}`, {
      status: response.status,
      details: payload,
    });
  }

  return payload;
}

async function readPayload(response: Response): Promise<unknown> {
  const contentType = response.headers.get("content-type") ?? "";
  const text = await response.text();
  if (text.length === 0) {
    return null;
  }

  if (contentType.includes("application/json")) {
    return JSON.parse(text) as unknown;
  }

  try {
    return JSON.parse(text) as unknown;
  } catch {
    return text;
  }
}

function render(title: string, payload: unknown, options: CliOptions): void {
  if (options.json) {
    console.log(JSON.stringify(payload, null, 2));
    return;
  }

  console.log(title);
  if (isRecord(payload)) {
    for (const [key, value] of Object.entries(payload)) {
      console.log(`${key}: ${formatValue(value)}`);
    }
    return;
  }

  console.log(formatValue(payload));
}

function printHelp(): void {
  console.log(`AgentsPay CLI

Usage:
  agentspay status [--base-url URL] [--json]
  agentspay demo [--sandbox|--live] [--json]
  agentspay balance [--base-url URL] [--json]
  agentspay endpoints list [--base-url URL] [--json]

Environment:
  AGENTSPAY_BASE_URL   Defaults to ${DEFAULT_BASE_URL}
  AGENTSPAY_API_KEY    Optional bearer token
`);
}

function formatValue(value: unknown): string {
  if (value === null || value === undefined) {
    return "";
  }
  if (typeof value === "string") {
    return value;
  }
  if (typeof value === "number" || typeof value === "boolean") {
    return String(value);
  }
  return JSON.stringify(value, null, 2);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
