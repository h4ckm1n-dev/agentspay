#!/usr/bin/env node
/**
 * agentspay — command-line interface for the local AgentsPay wallet.
 *
 * Wraps @agentspay/sdk-js. Every subcommand spawns the agentspay-mcp binary
 * once per call (same transport the SDK uses programmatically).
 *
 * Usage:
 *   agentspay balance
 *   agentspay pay-url <url> --max <usdc>
 *   agentspay set-budget --daily <usd> --per-call <usd>
 *   agentspay audit-log [--limit N]
 *   agentspay topup-info
 *
 * Global flags:
 *   --network <sandbox|solana-devnet|solana-mainnet>   (default: solana-devnet)
 *   --bin <path>           path to the agentspay-mcp binary
 *   --keypair <path>       override AGENTSPAY_KEYPAIR_PATH
 *   --json                 emit raw JSON instead of pretty output
 *   --debug                inherit subprocess stderr (the pretty banner)
 *   -h, --help             show help
 *   -v, --version          show version
 */

import {
  AgentsPayClient,
  AgentsPayError,
  BinaryNotFoundError,
  BudgetExceededError,
  InvalidInputError,
  PerCallCapExceededError,
  TransportTimeoutError,
  X402SettlementError,
  type AuditLogResponse,
  type BalanceResponse,
  type Network,
  type PayUrlResponse,
  type SetBudgetResponse,
  type TopupInfoResponse,
} from "@agentspay/sdk-js";

const VERSION = "0.2.0";

interface GlobalOptions {
  readonly network: Network;
  readonly mcpBinPath: string | undefined;
  readonly keypairPath: string | undefined;
  readonly json: boolean;
  readonly debug: boolean;
}

void run(process.argv.slice(2));

async function run(argv: ReadonlyArray<string>): Promise<void> {
  try {
    if (argv.length === 0 || argv[0] === "-h" || argv[0] === "--help") {
      printHelp();
      return;
    }
    if (argv[0] === "-v" || argv[0] === "--version") {
      console.log(VERSION);
      return;
    }

    const { command, args, globals } = parseArgs(argv);
    const client = new AgentsPayClient({
      network: globals.network,
      ...(globals.mcpBinPath !== undefined && { mcpBinPath: globals.mcpBinPath }),
      ...(globals.keypairPath !== undefined && { keypairPath: globals.keypairPath }),
      debug: globals.debug,
    });

    switch (command) {
      case "balance":
        await runBalance(client, globals);
        break;
      case "pay-url":
        await runPayUrl(client, args, globals);
        break;
      case "set-budget":
        await runSetBudget(client, args, globals);
        break;
      case "audit-log":
        await runAuditLog(client, args, globals);
        break;
      case "topup-info":
        await runTopupInfo(client, globals);
        break;
      default:
        fail(`unknown command: ${command}. Run 'agentspay --help'.`);
    }
  } catch (err) {
    handleError(err);
    process.exit(1);
  }
}

// ---------------------------------------------------------------------------
// Subcommands
// ---------------------------------------------------------------------------

async function runBalance(
  client: AgentsPayClient,
  globals: GlobalOptions,
): Promise<void> {
  const result = await client.balance();
  emit(result, globals.json, printBalance);
}

async function runPayUrl(
  client: AgentsPayClient,
  args: ReadonlyArray<string>,
  globals: GlobalOptions,
): Promise<void> {
  const positional = args.filter((a) => !a.startsWith("--"));
  const url = positional[0];
  if (url === undefined) {
    fail("pay-url requires a URL argument");
  }
  const max = readFlag(args, "--max") ?? readFlag(args, "--max-amount-usdc");
  if (max === undefined) {
    fail("pay-url requires --max <usdc>");
  }
  const result = await client.payUrl({ url, maxAmountUsdc: max });
  emit(result, globals.json, printPayUrl);
}

async function runSetBudget(
  client: AgentsPayClient,
  args: ReadonlyArray<string>,
  globals: GlobalOptions,
): Promise<void> {
  const dailyRaw = readFlag(args, "--daily");
  const perCallRaw = readFlag(args, "--per-call");
  if (dailyRaw === undefined || perCallRaw === undefined) {
    fail("set-budget requires --daily <usd> and --per-call <usd>");
  }
  const dailyUsd = Number(dailyRaw);
  const perCallUsd = Number(perCallRaw);
  if (!Number.isFinite(dailyUsd) || !Number.isFinite(perCallUsd)) {
    fail("--daily and --per-call must be numeric");
  }
  const result = await client.setBudget({ dailyUsd, perCallUsd });
  emit(result, globals.json, printSetBudget);
}

async function runAuditLog(
  client: AgentsPayClient,
  args: ReadonlyArray<string>,
  globals: GlobalOptions,
): Promise<void> {
  const limitRaw = readFlag(args, "--limit");
  const limit = limitRaw !== undefined ? Number(limitRaw) : undefined;
  if (limit !== undefined && !Number.isInteger(limit)) {
    fail("--limit must be an integer");
  }
  const result = await client.auditLog(limit !== undefined ? { limit } : {});
  emit(result, globals.json, printAuditLog);
}

async function runTopupInfo(
  client: AgentsPayClient,
  globals: GlobalOptions,
): Promise<void> {
  const result = await client.topupInfo();
  emit(result, globals.json, printTopupInfo);
}

// ---------------------------------------------------------------------------
// Pretty printers — match the spirit of services/mcp/src/pretty.rs
// ---------------------------------------------------------------------------

function printBalance(b: BalanceResponse): void {
  println(`Available     ${green(`${b.available_usdc} USDC`)}`);
  println(
    `Today's spend ${b.today_spent_usdc} USDC (cap ${b.daily_cap_usdc}, ${b.budget_remaining_today_usdc} left)`,
  );
  println(`Per-call cap  ${b.per_call_cap_usdc} USDC`);
  println(`Network       ${cyan(b.environment)}`);
  println(`Pubkey        ${dim(b.solana_pubkey)}`);
}

function printPayUrl(r: PayUrlResponse): void {
  if (r.payment_status === "paid") {
    println(`${green("Paid")} ${r.amount_charged_usdc} USDC for ${r.endpoint}`);
  } else {
    println(`${dim("No payment required.")} (${r.endpoint})`);
  }
  if (r.transaction !== "") {
    println(`Tx        ${dim(r.transaction)}`);
  }
  if (r.explorer_url !== "") {
    println(`Solscan   ${cyan(r.explorer_url)}`);
  }
  if (r.body !== "") {
    println(`Response  ${r.body}`);
  }
}

function printSetBudget(r: SetBudgetResponse): void {
  println(`${green("Budget updated")} (${dim(r.updated_at_rfc3339)})`);
  println(`  Daily      ${r.daily_usd.toFixed(2)} USDC`);
  println(`  Per-call   ${r.per_call_usd.toFixed(2)} USDC`);
}

function printAuditLog(r: AuditLogResponse): void {
  if (r.entries.length === 0) {
    println(dim("No audit entries."));
    return;
  }
  println(
    `${pad("ID", 12)}  ${pad("TIMESTAMP", 32)}  ${pad("TOOL", 22)}  ${pad("AMOUNT", 12)}  STATUS`,
  );
  for (const e of r.entries) {
    const amount = e.amount_usdc ? `${e.amount_usdc} USDC` : "-";
    const status = truncate(e.status, 60);
    const endpoint = e.endpoint ? dim(` (${e.endpoint})`) : "";
    println(
      `${pad(shortId(e.id), 12)}  ${pad(e.timestamp_rfc3339, 32)}  ${pad(e.tool, 22)}  ${pad(amount, 12)}  ${status}${endpoint}`,
    );
  }
  println(dim(`(${r.returned} of ${r.total} entries)`));
}

function truncate(s: string, width: number): string {
  if (s.length <= width) return s;
  return `${s.slice(0, width - 1)}…`;
}

function printTopupInfo(r: TopupInfoResponse): void {
  println(`Network        ${cyan(r.network)}`);
  println(`Pubkey         ${r.pubkey}`);
  println(`USDC faucet    ${cyan(r.faucet_url)}`);
  println(`SOL faucet     ${cyan(r.sol_faucet_url)}`);
  println("");
  println(r.instructions);
}

// ---------------------------------------------------------------------------
// Arg parsing
// ---------------------------------------------------------------------------

function parseArgs(argv: ReadonlyArray<string>): {
  readonly command: string;
  readonly args: ReadonlyArray<string>;
  readonly globals: GlobalOptions;
} {
  const command = argv[0] ?? "";
  const rest = argv.slice(1);

  const network = (readFlag(rest, "--network") as Network | undefined) ?? "solana-devnet";
  const mcpBinPath = readFlag(rest, "--bin");
  const keypairPath = readFlag(rest, "--keypair");
  const json = rest.includes("--json");
  const debug = rest.includes("--debug");

  return {
    command,
    args: rest,
    globals: { network, mcpBinPath, keypairPath, json, debug },
  };
}

function readFlag(args: ReadonlyArray<string>, name: string): string | undefined {
  const idx = args.indexOf(name);
  if (idx === -1 || idx === args.length - 1) return undefined;
  return args[idx + 1];
}

// ---------------------------------------------------------------------------
// Output helpers
// ---------------------------------------------------------------------------

function emit<T>(value: T, asJson: boolean, pretty: (v: T) => void): void {
  if (asJson) {
    println(JSON.stringify(value, null, 2));
  } else {
    pretty(value);
  }
}

function println(line: string): void {
  process.stdout.write(`${line}\n`);
}

function pad(s: string, width: number): string {
  if (s.length >= width) return s;
  return s + " ".repeat(width - s.length);
}

function shortId(id: string): string {
  if (id.length <= 8) return id;
  return `${id.slice(0, 8)}...`;
}

// ANSI helpers — true-color/256-color codes, fall back to plain if stdout is
// not a TTY or NO_COLOR is set.
function ansiEnabled(): boolean {
  if (process.env.NO_COLOR !== undefined) return false;
  if (process.env.FORCE_COLOR === "1") return true;
  return process.stdout.isTTY === true;
}

function wrap(code: string, text: string): string {
  return ansiEnabled() ? `\x1b[${code}m${text}\x1b[0m` : text;
}

function green(text: string): string {
  return wrap("38;5;79", text);
}
function cyan(text: string): string {
  return wrap("38;5;117", text);
}
function dim(text: string): string {
  return wrap("38;5;245", text);
}
function red(text: string): string {
  return wrap("38;5;203", text);
}

// ---------------------------------------------------------------------------
// Help + errors
// ---------------------------------------------------------------------------

function printHelp(): void {
  println(`agentspay ${VERSION} — local USDC wallet for AI agents

Usage:
  agentspay <command> [flags]

Commands:
  balance                                  Show current balance, budget, pubkey
  pay-url <url> --max <usdc>               Pay an x402-priced URL, return body
  set-budget --daily <usd> --per-call <usd>  Update spending caps
  audit-log [--limit N]                    Recent ledger rows
  topup-info                               Pubkey + faucet URLs

Global flags:
  --network <sandbox|solana-devnet|solana-mainnet>   default: solana-devnet
  --bin <path>           path to agentspay-mcp binary
  --keypair <path>       override AGENTSPAY_KEYPAIR_PATH
  --json                 raw JSON output for piping
  --debug                inherit subprocess stderr
  -h, --help             show this help
  -v, --version          ${VERSION}

Docs: https://agentspay.dev/docs`);
}

function fail(message: string): never {
  process.stderr.write(`${red("error")}: ${message}\n`);
  process.exit(2);
}

function handleError(err: unknown): void {
  if (err instanceof BinaryNotFoundError) {
    process.stderr.write(`${red("error")}: ${err.message}\n`);
    return;
  }
  if (err instanceof TransportTimeoutError) {
    process.stderr.write(`${red("timeout")}: ${err.message}\n`);
    return;
  }
  if (err instanceof PerCallCapExceededError) {
    process.stderr.write(`${red("per-call cap exceeded")}: ${err.message}\n`);
    return;
  }
  if (err instanceof BudgetExceededError) {
    process.stderr.write(`${red("daily budget exceeded")}: ${err.message}\n`);
    return;
  }
  if (err instanceof X402SettlementError) {
    process.stderr.write(`${red("settlement failed")}: ${err.message}\n`);
    return;
  }
  if (err instanceof InvalidInputError) {
    process.stderr.write(`${red("invalid input")}: ${err.message}\n`);
    return;
  }
  if (err instanceof AgentsPayError) {
    process.stderr.write(`${red(err.code)}: ${err.message}\n`);
    return;
  }
  process.stderr.write(`${red("error")}: ${String(err)}\n`);
}
