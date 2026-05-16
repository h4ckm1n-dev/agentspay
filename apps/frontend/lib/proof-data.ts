export interface ProofRecord {
  readonly symbol: string;
  readonly amountUsdc: string;
  readonly signature: string;
  readonly context: string;
  readonly payer?: string;
  readonly payee?: string;
}

export const PROOF_RECORDS: readonly ProofRecord[] = [
  {
    symbol: "AAPL",
    amountUsdc: "0.10",
    signature:
      "4pGRMVgu7j5itCs7Vf6G9FTQW2Q1B2SjCEKHszLjvF9eVagWvtWq8aJWuYz1JNpBQr4CsbYRXSb9aWAu5hv6jYau",
    context: "Native MCP smoke test",
    payer: "GmBDzsdcPBNpeGchxX2GkZTKYtuCKnj7wyHiYaL9zPEm",
    payee: "HE5JxfV1VVxrjXNW9ybopevF9uQwyWmJZrYXZxiv7Btv",
  },
  {
    symbol: "GOOG",
    amountUsdc: "0.10",
    signature:
      "3EUyjsdN7Y2ZHTUFMaNn3Y3TyMsGcK673Bis5oMw49RgTPGXhUJJiRYyG2JYrkkQypfszJH9FuRBPScTmXh2BFJU",
    context: "Web-shim HTTP bridge",
  },
  {
    symbol: "GOOG",
    amountUsdc: "0.10",
    signature:
      "ogEatB8NTZ3KiLufnWwVjU25jBWygwLNdhNJqKHZPSftgrWUWBdD5P1JQ6kDXVj6HzQnPXb55bcPjCGWFmAFFkJ",
    context: "Next.js to shim to MCP to paid endpoint",
  },
];

export function solscanUrl(signature: string): string {
  return `https://solscan.io/tx/${signature}?cluster=devnet`;
}

export function shortSignature(signature: string): string {
  if (signature.length <= 14) {
    return signature;
  }
  return `${signature.slice(0, 6)}...${signature.slice(-6)}`;
}
