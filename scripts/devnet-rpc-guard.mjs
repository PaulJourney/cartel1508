import { Connection } from "@solana/web3.js";

const nativeFetch = globalThis.fetch?.bind(globalThis);

if (!nativeFetch) {
  throw new Error("Node global fetch is unavailable; cannot install Devnet RPC guard");
}

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const MIN_INTERVAL_MS = Number(process.env.DEVNET_RPC_MIN_INTERVAL_MS || 450);
const MAX_RETRIES = Number(process.env.DEVNET_RPC_MAX_RETRIES || 18);
const NULL_ACCOUNT_RETRIES = Number(process.env.DEVNET_NULL_ACCOUNT_RETRIES || 12);
const BLOCKHASH_RETRIES = Number(process.env.DEVNET_BLOCKHASH_RETRIES || 0);
const TRANSACTION_BLOCKHASH_RETRIES = Number(
  process.env.DEVNET_TRANSACTION_BLOCKHASH_RETRIES || 5,
);
const WEB3_RPC_MIN_INTERVAL_MS = Number(
  process.env.DEVNET_WEB3_RPC_MIN_INTERVAL_MS || 650,
);
const WEB3_RPC_429_RETRIES = Number(
  process.env.DEVNET_WEB3_RPC_429_RETRIES || 8,
);

let queue = Promise.resolve();
let nextRequestAt = 0;
let web3RpcQueue = Promise.resolve();
let web3RpcNextRequestAt = 0;

function isGuardedRpc(input) {
  const url = typeof input === "string" ? input : input?.url || String(input || "");
  return url.includes("api.devnet.solana.com");
}

function rpcPayload(init) {
  const body = init?.body;
  if (typeof body !== "string") return null;
  try {
    return JSON.parse(body);
  } catch {
    return null;
  }
}

function rpcMethod(init) {
  return rpcPayload(init)?.method || null;
}

function withAlignedPreflightCommitment(init, method) {
  if (!["sendTransaction", "simulateTransaction"].includes(method)) return init;

  const payload = rpcPayload(init);
  if (!payload || !Array.isArray(payload.params)) return init;

  const config = payload.params[1];
  if (config?.preflightCommitment) return init;

  payload.params[1] = {
    ...(config && typeof config === "object" ? config : {}),
    preflightCommitment: "confirmed",
  };

  return {
    ...init,
    body: JSON.stringify(payload),
  };
}

function isBlockhashNotFoundError(error) {
  const message = String(error?.message || error || "");
  const transactionMessage = String(error?.transactionMessage || "");
  return /blockhash not found/i.test(`${message} ${transactionMessage}`);
}

function isRateLimitError(error) {
  return /(^|\s)429(\s|$)|too many requests|rate limit/i.test(
    String(error?.message || error || ""),
  );
}

async function withWeb3RpcPacing(invoke, method = "unknown") {
  let release;
  const previous = web3RpcQueue;
  web3RpcQueue = new Promise((resolve) => {
    release = resolve;
  });
  await previous;

  try {
    const spacing = Math.max(0, web3RpcNextRequestAt - Date.now());
    if (spacing > 0) await sleep(spacing);

    let lastError;
    for (let attempt = 0; attempt <= WEB3_RPC_429_RETRIES; attempt += 1) {
      web3RpcNextRequestAt = Date.now() + WEB3_RPC_MIN_INTERVAL_MS;
      try {
        return await invoke();
      } catch (error) {
        lastError = error;
        if (!isRateLimitError(error) || attempt === WEB3_RPC_429_RETRIES) {
          throw error;
        }
        const retryNumber = attempt + 1;
        const delay = Math.min(12_000, 1_000 * (2 ** Math.min(attempt, 3)));
        console.warn(
          `Devnet web3 RPC ${method} rate-limited; retry ${retryNumber}/${WEB3_RPC_429_RETRIES} after ${delay}ms`,
        );
        await sleep(delay);
      }
    }
    throw lastError;
  } finally {
    release();
  }
}

// web3.js v1.x assigns _rpcRequest and _rpcBatchRequest on each Connection
// instance inside the constructor. Installing inherited accessors before any
// Connection is constructed lets us wrap those internal request functions and
// pace every JSON-RPC call, including calls that do not use globalThis.fetch.
const RPC_REQUEST = Symbol("devnet-rpc-request");
const RPC_BATCH_REQUEST = Symbol("devnet-rpc-batch-request");

Object.defineProperty(Connection.prototype, "_rpcRequest", {
  configurable: true,
  get() {
    return this[RPC_REQUEST];
  },
  set(value) {
    if (typeof value !== "function") {
      this[RPC_REQUEST] = value;
      return;
    }
    this[RPC_REQUEST] = (method, args) =>
      withWeb3RpcPacing(() => value(method, args), method);
  },
});

Object.defineProperty(Connection.prototype, "_rpcBatchRequest", {
  configurable: true,
  get() {
    return this[RPC_BATCH_REQUEST];
  },
  set(value) {
    if (typeof value !== "function") {
      this[RPC_BATCH_REQUEST] = value;
      return;
    }
    this[RPC_BATCH_REQUEST] = (requests) =>
      withWeb3RpcPacing(() => value(requests), "batch");
  },
});

// web3.js obtains and signs a legacy Transaction inside Connection.sendTransaction.
// Retrying the raw JSON-RPC payload would reuse the stale signed blockhash, so that
// cannot recover. Retrying Connection.sendTransaction with the original signers
// forces web3.js to fetch a fresh blockhash and re-sign the transaction.
const nativeSendTransaction = Connection.prototype.sendTransaction;
Connection.prototype.sendTransaction = async function guardedSendTransaction(
  transaction,
  signersOrOptions,
  options,
) {
  // Legacy transactions pass a signer array. Versioned transactions are already
  // signed and cannot be safely regenerated here, so leave them untouched.
  if (!Array.isArray(signersOrOptions)) {
    return nativeSendTransaction.call(this, transaction, signersOrOptions, options);
  }

  let lastError;
  for (let attempt = 0; attempt <= TRANSACTION_BLOCKHASH_RETRIES; attempt += 1) {
    try {
      return await nativeSendTransaction.call(this, transaction, signersOrOptions, options);
    } catch (error) {
      lastError = error;
      if (!isBlockhashNotFoundError(error) || attempt === TRANSACTION_BLOCKHASH_RETRIES) {
        throw error;
      }
      const retryNumber = attempt + 1;
      console.warn(
        `Devnet transaction retrying with fresh blockhash (${retryNumber}/${TRANSACTION_BLOCKHASH_RETRIES})`,
      );
      await sleep(750 + (retryNumber * 500));
    }
  }
  throw lastError;
};

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
    const requestInit = withAlignedPreflightCommitment(init, method);
    let nullAccountRetries = 0;
    let blockhashRetries = 0;
    let lastResponse;
    let lastError;

    for (let attempt = 0; attempt <= MAX_RETRIES; attempt += 1) {
      nextRequestAt = Date.now() + MIN_INTERVAL_MS;
      try {
        const response = await nativeFetch(input, requestInit);
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
          console.warn(
            `Devnet RPC guard retrying transient null account (${nullAccountRetries}/${NULL_ACCOUNT_RETRIES})`,
          );
          await sleep(nullDelay);
          continue;
        } else if (await isTransientBlockhashError(response, method)) {
          // Do not retry the same signed RPC payload. Let Connection.sendTransaction
          // see the error so the prototype wrapper above can regenerate and re-sign.
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
  `Devnet RPC guard enabled: fetchMinInterval=${MIN_INTERVAL_MS}ms web3RpcMinInterval=${WEB3_RPC_MIN_INTERVAL_MS}ms maxRetries=${MAX_RETRIES} web3Rpc429Retries=${WEB3_RPC_429_RETRIES} nullAccountRetries=${NULL_ACCOUNT_RETRIES} transactionBlockhashRetries=${TRANSACTION_BLOCKHASH_RETRIES} preflightCommitment=confirmed`,
);
