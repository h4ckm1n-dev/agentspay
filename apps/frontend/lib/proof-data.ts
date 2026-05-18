export function solscanUrl(signature: string): string {
  return `https://solscan.io/tx/${signature}?cluster=devnet`;
}

export function shortSignature(signature: string): string {
  if (signature.length <= 14) {
    return signature;
  }
  return `${signature.slice(0, 6)}...${signature.slice(-6)}`;
}
