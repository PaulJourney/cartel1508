const nativeFetch = globalThis.fetch?.bind(globalThis);

if (!nativeFetch) {
  throw new Error("Node global fetch is unavailable; cannot install Devnet RPC guard");
}

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const MIN_INTERVAL_MS = Number(process.env.DEVNET_RPC_MIN_INTERVAL_MS || 450);
const MAX_RETRIES = Number(process.env.DEVNET_RPC_MAX_RETRIES || 9);
const NULL_ACCOUNT_RETRIES = Number(process.env.DEVNET_NULL_ACCOUNT_RETRIES || 4);
const BLOCKHASH_RETRIES = Number(process.env.DEVNET_BLOCKHASH_RETRIES || 5);

let queue = Promise.resolve();
let nextRequestAt = 0;

function isGuardedRpc(input) {
  const url = typeof input === "string" ? input : input?.url || String(input || "");
  return url.includes("api.devnet.solana.com");
}

function rpcMethod(init) {
  const body = init?.body;
  if (typeof body !== "string") return null;
  try {
    return JSON.parse(body)?.method || null;
  } catch {
    return null;
  }
}

async function isTransientNullAccount(response, method) {
  if (method !== "getAccountInfo" || response.status !== 200) return false;
  try {
    const payload = await response.clone().json();
    return payload?.result?.value === null && !payload?.error;
  } catch {
    return false;
  }
}

async function isTransientBlockhashError(response, method) {
  if (method !== "sendTransaction" || response.status !== 200) return false;
  try {
    const payload = await response.clone().json();
    const message = String(payload?.error?.message || "");
    const dataMessage = String(payload?.error?.data?.err || "");
    return /blockhash not found/i.test(`${message} ${dataMessage}`);
  } catch {
    return false;
  }
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

    const method = rpcMethod(init);
    let nullAccountRetries = 0;
    let blockhashRetries = 0;
    let lastResponse;
    let lastError;

    for (let attempt = 0; attempt <= MAX_RETRIES; attempt += 1) {
      nextRequestAt = Date.now() + MIN_INTERVAL_MS;
      try {
        const response = await nativeFetch(input, init);
        lastResponse = response;

        if ([429, 502, 503, 504].includes(response.status)) {
          if (attempt === MAX_RETRIES) return response;
          try {
            await response.text();
          } catch {
            // Ignore body-read errors on infrastructure responses.
          }
        } else if (await isTransientNullAccount(response, method)) {
          if (nullAccountRetries >= NULL_ACCOUNT_RETRIES) {
            return response;
          }
          nullAccountRetries += 1;
          const nullDelay = 500 + (nullAccountRetries * 350);
          await sleep(nullDelay);
          continue;
        } else if (await isTransientBlockhashError(response, method)) {
          if (blockhashRetries >= BLOCKHASH_RETRIES) {
            return response;
          }
          blockhashRetries += 1;
          const blockhashDelay = 650 + (blockhashRetries * 450);
          console.warn(
            `Devnet RPC guard retrying transient Blockhash not found (${blockhashRetries}/${BLOCKHASH_RETRIES})`,
          );
          await sleep(blockhashDelay);
          continue;
        } else {
          return response;
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
  `Devnet RPC guard enabled: minInterval=${MIN_INTERVAL_MS}ms maxRetries=${MAX_RETRIES} nullAccountRetries=${NULL_ACCOUNT_RETRIES} blockhashRetries=${BLOCKHASH_RETRIES}`,
);
