const nativeFetch = globalThis.fetch?.bind(globalThis);

if (!nativeFetch) {
  throw new Error("Node global fetch is unavailable; cannot install Devnet RPC guard");
}

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const MIN_INTERVAL_MS = Number(process.env.DEVNET_RPC_MIN_INTERVAL_MS || 450);
const MAX_RETRIES = Number(process.env.DEVNET_RPC_MAX_RETRIES || 9);

let queue = Promise.resolve();
let nextRequestAt = 0;

function isGuardedRpc(input) {
  const url = typeof input === "string" ? input : input?.url || String(input || "");
  return url.includes("api.devnet.solana.com");
}

globalThis.fetch = async function guardedDevnetFetch(input, init) {
  if (!isGuardedRpc(input)) {
    return nativeFetch(input, init);
  }

  let release;
  const previous = queue;
  queue = new Promise((resolve) => {
    release = resolve;
  });
  await previous;

  try {
    const spacing = Math.max(0, nextRequestAt - Date.now());
    if (spacing > 0) await sleep(spacing);

    let lastResponse;
    let lastError;
    for (let attempt = 0; attempt <= MAX_RETRIES; attempt += 1) {
      nextRequestAt = Date.now() + MIN_INTERVAL_MS;
      try {
        const response = await nativeFetch(input, init);
        lastResponse = response;
        if (![429, 502, 503, 504].includes(response.status)) {
          return response;
        }

        if (attempt === MAX_RETRIES) return response;
        try {
          await response.text();
        } catch {
          // Ignore body-read errors on infrastructure responses.
        }
      } catch (error) {
        lastError = error;
        if (attempt === MAX_RETRIES) throw error;
      }

      const exponential = Math.min(10_000, 650 * (2 ** Math.min(attempt, 4)));
      const jitter = Math.floor(Math.random() * 250);
      await sleep(exponential + jitter);
    }

    if (lastResponse) return lastResponse;
    throw lastError || new Error("Devnet RPC guard exhausted without response");
  } finally {
    release();
  }
};

console.log(
  `Devnet RPC guard enabled: minInterval=${MIN_INTERVAL_MS}ms maxRetries=${MAX_RETRIES}`,
);
