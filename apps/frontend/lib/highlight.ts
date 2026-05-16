import type { ReactNode } from "react";
import { createElement, Fragment } from "react";

/* ---------------------------------------------------------------------------
 * JSON highlighter
 *
 * A tiny recursive-descent tokenizer for JSON. Returns an array of <span>
 * elements that map to the `text-syntax-*` Tailwind tokens. Falls back to the
 * raw text (wrapped in a Fragment) when the input is not valid JSON.
 * ------------------------------------------------------------------------- */

type JsonTokenKind =
  | "punct"
  | "key"
  | "string"
  | "number"
  | "bool"
  | "null"
  | "whitespace"
  // domain-aware kinds (post-processing refinement only)
  | "signature"
  | "pubkey"
  | "usdc"
  | "timestamp"
  | "uuid"
  | "url";

interface JsonToken {
  kind: JsonTokenKind;
  text: string;
}

const KIND_TO_CLASS: Record<Exclude<JsonTokenKind, "whitespace">, string> = {
  punct: "text-syntax-punct",
  key: "text-syntax-key",
  string: "text-syntax-string",
  number: "text-syntax-number",
  bool: "text-syntax-bool",
  null: "text-syntax-null",
  signature: "text-syntax-signature font-medium",
  pubkey: "text-syntax-pubkey",
  usdc: "text-syntax-usdc font-medium",
  timestamp: "text-syntax-timestamp",
  uuid: "text-syntax-uuid",
  url: "text-syntax-url underline decoration-dotted underline-offset-2",
};

/* ---------------------------------------------------------------------------
 * Domain refinement
 *
 * Post-processes the raw token list, upgrading `string`/`number` tokens to
 * one of six domain-aware kinds (signature, pubkey, usdc, timestamp, uuid,
 * url) based on content shape + (for usdc) the most recent JSON key context.
 * ------------------------------------------------------------------------- */

const BASE58_RE = /^[1-9A-HJ-NP-Za-km-z]+$/;
const URL_RE = /^https?:\/\//;
const UUID_RE =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
const RFC3339_RE = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/;
const USDC_KEY_RE = /usdc|usd|amount|balance|price|cap|spent|value|cost/i;
const USDC_NUM_RE = /^-?\d+(\.\d+)?$/;
const USDC_STR_RE = /^\d+\.\d{1,8}$/;

function detectDomain(
  rawText: string,
  fallback: "string" | "number",
  recentKey: string | null,
): JsonTokenKind {
  const inner = fallback === "string" ? rawText.slice(1, -1) : rawText;

  if (fallback === "string") {
    if (URL_RE.test(inner)) return "url";
    if (UUID_RE.test(inner)) return "uuid";
    if (RFC3339_RE.test(inner)) return "timestamp";
    if (BASE58_RE.test(inner)) {
      if (inner.length >= 86 && inner.length <= 88) return "signature";
      if (inner.length >= 32 && inner.length <= 44) return "pubkey";
    }
  }

  if (recentKey && USDC_KEY_RE.test(recentKey)) {
    if (fallback === "number" && USDC_NUM_RE.test(rawText)) return "usdc";
    if (fallback === "string" && USDC_STR_RE.test(inner)) return "usdc";
  }

  return fallback;
}

function refineTokens(raw: JsonToken[]): JsonToken[] {
  let recentKey: string | null = null;
  const refined: JsonToken[] = [];
  for (const tok of raw) {
    if (tok.kind === "key") {
      recentKey = tok.text.replace(/^"|"$/g, "");
      refined.push(tok);
      continue;
    }
    if (tok.kind === "string" || tok.kind === "number") {
      const refinedKind = detectDomain(tok.text, tok.kind, recentKey);
      refined.push({ kind: refinedKind, text: tok.text });
      recentKey = null;
      continue;
    }
    if (
      tok.kind === "punct" &&
      (tok.text === "," || tok.text === "}" || tok.text === "]")
    ) {
      recentKey = null;
    }
    refined.push(tok);
  }
  return refined;
}

class JsonTokenizer {
  private pos = 0;
  private readonly src: string;
  private readonly out: JsonToken[] = [];

  constructor(src: string) {
    this.src = src;
  }

  tokenize(): JsonToken[] {
    this.value();
    this.skipWhitespace();
    if (this.pos !== this.src.length) {
      throw new Error("trailing input");
    }
    return this.out;
  }

  private peek(): string {
    return this.src[this.pos] ?? "";
  }

  private skipWhitespace(): void {
    const start = this.pos;
    while (this.pos < this.src.length && /\s/.test(this.src[this.pos]!)) {
      this.pos++;
    }
    if (this.pos > start) {
      this.out.push({
        kind: "whitespace",
        text: this.src.slice(start, this.pos),
      });
    }
  }

  private value(): void {
    this.skipWhitespace();
    const c = this.peek();
    if (c === "{") this.object();
    else if (c === "[") this.array();
    else if (c === '"') this.readString("string");
    else if (c === "t" || c === "f") this.readBool();
    else if (c === "n") this.readNull();
    else if (c === "-" || (c >= "0" && c <= "9")) this.readNumber();
    else throw new Error(`unexpected character "${c}" at ${this.pos}`);
  }

  private object(): void {
    this.consumePunct("{");
    this.skipWhitespace();
    if (this.peek() === "}") {
      this.consumePunct("}");
      return;
    }
    while (true) {
      this.skipWhitespace();
      this.readString("key");
      this.skipWhitespace();
      this.consumePunct(":");
      this.value();
      this.skipWhitespace();
      if (this.peek() === ",") {
        this.consumePunct(",");
        continue;
      }
      break;
    }
    this.skipWhitespace();
    this.consumePunct("}");
  }

  private array(): void {
    this.consumePunct("[");
    this.skipWhitespace();
    if (this.peek() === "]") {
      this.consumePunct("]");
      return;
    }
    while (true) {
      this.value();
      this.skipWhitespace();
      if (this.peek() === ",") {
        this.consumePunct(",");
        continue;
      }
      break;
    }
    this.skipWhitespace();
    this.consumePunct("]");
  }

  private consumePunct(expect: string): void {
    if (this.peek() !== expect) {
      throw new Error(`expected "${expect}" at ${this.pos}`);
    }
    this.out.push({ kind: "punct", text: expect });
    this.pos++;
  }

  private readString(asKind: "key" | "string"): void {
    if (this.peek() !== '"') throw new Error(`expected string at ${this.pos}`);
    const start = this.pos;
    this.pos++; // opening "
    while (this.pos < this.src.length) {
      const ch = this.src[this.pos]!;
      if (ch === "\\") {
        this.pos += 2;
        continue;
      }
      if (ch === '"') {
        this.pos++;
        const text = this.src.slice(start, this.pos);
        this.out.push({ kind: asKind, text });
        return;
      }
      this.pos++;
    }
    throw new Error("unterminated string");
  }

  private readBool(): void {
    if (this.src.startsWith("true", this.pos)) {
      this.out.push({ kind: "bool", text: "true" });
      this.pos += 4;
      return;
    }
    if (this.src.startsWith("false", this.pos)) {
      this.out.push({ kind: "bool", text: "false" });
      this.pos += 5;
      return;
    }
    throw new Error(`expected bool at ${this.pos}`);
  }

  private readNull(): void {
    if (this.src.startsWith("null", this.pos)) {
      this.out.push({ kind: "null", text: "null" });
      this.pos += 4;
      return;
    }
    throw new Error(`expected null at ${this.pos}`);
  }

  private readNumber(): void {
    const re = /-?\d+(\.\d+)?([eE][+-]?\d+)?/y;
    re.lastIndex = this.pos;
    const m = re.exec(this.src);
    if (!m) throw new Error(`expected number at ${this.pos}`);
    this.out.push({ kind: "number", text: m[0] });
    this.pos += m[0].length;
  }
}

function looksLikeJson(raw: string): boolean {
  const trimmed = raw.trimStart();
  return trimmed.startsWith("{") || trimmed.startsWith("[");
}

export function highlightJson(raw: string): ReactNode {
  if (!looksLikeJson(raw)) {
    return createElement(Fragment, null, raw);
  }
  let tokens: JsonToken[];
  try {
    tokens = new JsonTokenizer(raw).tokenize();
  } catch {
    return createElement(Fragment, null, raw);
  }
  tokens = refineTokens(tokens);
  const children: ReactNode[] = tokens.map((tok, i) => {
    if (tok.kind === "whitespace") {
      return createElement(Fragment, { key: i }, tok.text);
    }
    return createElement(
      "span",
      { key: i, className: KIND_TO_CLASS[tok.kind] },
      tok.text,
    );
  });
  return createElement(Fragment, null, ...children);
}

/* ---------------------------------------------------------------------------
 * Shell highlighter
 *
 * Splits on whitespace and colors:
 *   - first non-flag -> cmd (bold white)
 *   - --flag / -p    -> key (cyan)
 *   - everything else (args, identifiers) -> string (green)
 *   - inline : or =  -> punct
 *
 * Returns a fragment of inline spans. The wrapping element decides whether
 * the leading "$ " prompt sigil is emitted.
 * ------------------------------------------------------------------------- */

type ShellTokenKind = "cmd" | "flag" | "arg" | "punct" | "ws";

interface ShellToken {
  kind: ShellTokenKind;
  text: string;
}

const SHELL_KIND_TO_CLASS: Record<Exclude<ShellTokenKind, "ws">, string> = {
  cmd: "text-syntax-cmd font-semibold",
  flag: "text-syntax-key",
  arg: "text-syntax-string",
  punct: "text-syntax-punct",
};

function tokenizeShell(raw: string): ShellToken[] {
  const out: ShellToken[] = [];
  const words = raw.split(/(\s+)/); // keep whitespace
  let seenCmd = false;
  for (const w of words) {
    if (w.length === 0) continue;
    if (/^\s+$/.test(w)) {
      out.push({ kind: "ws", text: w });
      continue;
    }
    if (w.startsWith("-")) {
      out.push({ kind: "flag", text: w });
      continue;
    }
    if (!seenCmd) {
      out.push({ kind: "cmd", text: w });
      seenCmd = true;
      continue;
    }
    // arg may contain : or = punctuation we want to dim
    const parts = w.split(/([:=])/);
    for (const p of parts) {
      if (p === "") continue;
      if (p === ":" || p === "=") {
        out.push({ kind: "punct", text: p });
      } else {
        out.push({ kind: "arg", text: p });
      }
    }
  }
  return out;
}

export function highlightShell(raw: string): ReactNode {
  const tokens = tokenizeShell(raw);
  const children: ReactNode[] = tokens.map((tok, i) => {
    if (tok.kind === "ws") {
      return createElement(Fragment, { key: i }, tok.text);
    }
    return createElement(
      "span",
      { key: i, className: SHELL_KIND_TO_CLASS[tok.kind] },
      tok.text,
    );
  });
  return createElement(Fragment, null, ...children);
}
