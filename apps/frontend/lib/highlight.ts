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
  | "whitespace";

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
};

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
