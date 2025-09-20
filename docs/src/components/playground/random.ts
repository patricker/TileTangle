export function randomSeed(): number {
  // Try to use cryptographically strong randomness when available.
  if (typeof globalThis !== 'undefined') {
    const cryptoObj = (globalThis as Record<string, unknown>).crypto as
      | {getRandomValues?: (array: Uint32Array) => void}
      | undefined;
    if (cryptoObj && typeof cryptoObj.getRandomValues === 'function') {
      const buf = new Uint32Array(1);
      cryptoObj.getRandomValues(buf);
      const value = buf[0] >>> 0;
      return value === 0 ? 1 : value;
    }
  }
  // Fall back to Math.random; ensure non-zero so we avoid repeated seeds.
  const fallback = Math.floor(Math.random() * 0xffffffff) >>> 0;
  return fallback === 0 ? 1 : fallback;
}
