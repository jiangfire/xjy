const TEXT_ENCODER = new TextEncoder();
const TEXT_DECODER = new TextDecoder();
const SEP = TEXT_ENCODER.encode("|");

function base64UrlToBytes(input) {
  const base64 = input.replace(/-/g, "+").replace(/_/g, "/");
  const pad = (4 - (base64.length % 4)) % 4;
  const padded = base64 + "=".repeat(pad);
  const bin = atob(padded);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) {
    out[i] = bin.charCodeAt(i);
  }
  return out;
}

function bytesToBase64(input) {
  let binary = "";
  const chunkSize = 0x8000;
  for (let i = 0; i < input.length; i += chunkSize) {
    const chunk = input.subarray(i, i + chunkSize);
    binary += String.fromCharCode(...chunk);
  }
  return btoa(binary);
}

function concatBytes(parts) {
  let total = 0;
  for (const p of parts) total += p.length;
  const out = new Uint8Array(total);
  let offset = 0;
  for (const p of parts) {
    out.set(p, offset);
    offset += p.length;
  }
  return out;
}

function int32Le(value) {
  const buf = new ArrayBuffer(4);
  const view = new DataView(buf);
  view.setInt32(0, Number(value), true);
  return new Uint8Array(buf);
}

function int64Le(value) {
  const buf = new ArrayBuffer(8);
  const view = new DataView(buf);
  view.setBigInt64(0, BigInt(Math.trunc(value)), true);
  return new Uint8Array(buf);
}

function hasLeadingZeroBits(bytes, bits) {
  const fullBytes = Math.floor(bits / 8);
  const remBits = bits % 8;
  const needLen = fullBytes + (remBits > 0 ? 1 : 0);
  if (bytes.length < needLen) return false;

  for (let i = 0; i < fullBytes; i++) {
    if (bytes[i] !== 0) return false;
  }

  if (remBits === 0) return true;
  const mask = (0xff << (8 - remBits)) & 0xff;
  return (bytes[fullBytes] & mask) === 0;
}

function mustNumber(raw, key) {
  const v = raw[key];
  if (typeof v !== "number" || !Number.isFinite(v)) {
    throw new Error(`pow_token payload 字段无效: ${key}`);
  }
  return v;
}

function mustString(raw, key) {
  const v = raw[key];
  if (typeof v !== "string") {
    throw new Error(`pow_token payload 字段无效: ${key}`);
  }
  return v;
}

export function decodePowChallenge(powToken) {
  const [payloadB64] = String(powToken).split(".");
  if (!payloadB64) {
    throw new Error("Invalid pow_token");
  }

  const payloadBytes = base64UrlToBytes(payloadB64);
  const payload = JSON.parse(TEXT_DECODER.decode(payloadBytes));
  return {
    v: mustNumber(payload, "v"),
    action: mustString(payload, "action"),
    target_type: mustString(payload, "target_type"),
    target_id: mustNumber(payload, "target_id"),
    user_id: mustNumber(payload, "user_id"),
    issued_at: mustNumber(payload, "issued_at"),
    expires_at: mustNumber(payload, "expires_at"),
    difficulty: mustNumber(payload, "difficulty"),
    salt: mustString(payload, "salt"),
  };
}

function buildPowPrefix(challenge) {
  return concatBytes([
    TEXT_ENCODER.encode(challenge.action),
    SEP,
    TEXT_ENCODER.encode(challenge.target_type),
    SEP,
    int32Le(challenge.target_id),
    SEP,
    int32Le(challenge.user_id),
    SEP,
    int64Le(challenge.issued_at),
    SEP,
    int64Le(challenge.expires_at),
    SEP,
    Uint8Array.of(challenge.difficulty & 0xff),
    SEP,
    TEXT_ENCODER.encode(challenge.salt),
    SEP,
  ]);
}

async function solveSingleThread(
  prefix,
  difficulty,
  startNonce,
  maxIterations,
  reportEvery,
  onProgress,
  signal
) {
  let attempts = 0;
  for (let i = 0; i < maxIterations; i++) {
    if (signal?.aborted) {
      throw new Error("PoW solve aborted");
    }

    const nonce = String(startNonce + i);
    const nonceBytes = TEXT_ENCODER.encode(nonce);
    const input = new Uint8Array(prefix.length + nonceBytes.length);
    input.set(prefix);
    input.set(nonceBytes, prefix.length);

    const digestBuf = await crypto.subtle.digest("SHA-256", input);
    const digest = new Uint8Array(digestBuf);
    attempts++;

    if (hasLeadingZeroBits(digest, difficulty)) {
      return nonce;
    }

    if (onProgress && attempts % reportEvery === 0) {
      onProgress(attempts);
    }
  }

  throw new Error(
    `未在 ${maxIterations} 次内找到解（difficulty=${difficulty}）`
  );
}

const WORKER_SOURCE = `
const ENC = new TextEncoder();

function hasLeadingZeroBits(bytes, bits) {
  const fullBytes = Math.floor(bits / 8);
  const remBits = bits % 8;
  const needLen = fullBytes + (remBits > 0 ? 1 : 0);
  if (bytes.length < needLen) return false;
  for (let i = 0; i < fullBytes; i++) {
    if (bytes[i] !== 0) return false;
  }
  if (remBits === 0) return true;
  const mask = (0xff << (8 - remBits)) & 0xff;
  return (bytes[fullBytes] & mask) === 0;
}

function base64ToBytes(base64) {
  const bin = atob(base64);
  const out = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) {
    out[i] = bin.charCodeAt(i);
  }
  return out;
}

let stopped = false;

self.onmessage = async (event) => {
  const msg = event.data;
  if (msg.type === "stop") {
    stopped = true;
    return;
  }
  if (msg.type !== "start") return;

  try {
    const {
      prefixBase64,
      difficulty,
      startNonce,
      maxIterations,
      offset,
      stride,
      reportEvery,
    } = msg;

    const prefix = base64ToBytes(prefixBase64);
    let localAttempts = 0;

    for (let i = offset; i < maxIterations; i += stride) {
      if (stopped) return;

      const nonce = String(startNonce + i);
      const nonceBytes = ENC.encode(nonce);
      const input = new Uint8Array(prefix.length + nonceBytes.length);
      input.set(prefix);
      input.set(nonceBytes, prefix.length);

      const digestBuf = await crypto.subtle.digest("SHA-256", input);
      const digest = new Uint8Array(digestBuf);
      localAttempts++;

      if (hasLeadingZeroBits(digest, difficulty)) {
        self.postMessage({ type: "found", nonce });
        return;
      }

      if (localAttempts % reportEvery === 0) {
        self.postMessage({ type: "progress", attempts: reportEvery });
      }
    }

    const rem = localAttempts % reportEvery;
    if (rem > 0) {
      self.postMessage({ type: "progress", attempts: rem });
    }
    self.postMessage({ type: "done" });
  } catch (err) {
    const message = err && err.message ? err.message : String(err);
    self.postMessage({ type: "error", error: message });
  }
};
`;

function getDefaultWorkerCount() {
  const hc = Number(globalThis.navigator?.hardwareConcurrency || 4);
  return Math.max(1, hc - 1);
}

export async function solvePowNonceBrowser(powToken, options = {}) {
  const challenge = decodePowChallenge(powToken);
  const prefix = buildPowPrefix(challenge);

  const workerCount = Math.max(
    1,
    Math.floor(options.workers ?? getDefaultWorkerCount())
  );
  const startNonce = Math.trunc(options.startNonce ?? 0);
  const maxIterations = Math.max(
    1,
    Math.trunc(options.maxIterations ?? 50_000_000)
  );
  const reportEvery = Math.max(1, Math.trunc(options.reportEvery ?? 50_000));

  if (workerCount === 1 || typeof Worker === "undefined") {
    return solveSingleThread(
      prefix,
      challenge.difficulty,
      startNonce,
      maxIterations,
      reportEvery,
      options.onProgress,
      options.signal
    );
  }

  return new Promise((resolve, reject) => {
    let settled = false;
    let totalAttempts = 0;
    let doneWorkers = 0;
    const workers = [];
    const workerBlobUrl = URL.createObjectURL(
      new Blob([WORKER_SOURCE], { type: "text/javascript" })
    );

    const cleanup = () => {
      for (const w of workers) {
        try {
          w.postMessage({ type: "stop" });
          w.terminate();
        } catch (_) {}
      }
      URL.revokeObjectURL(workerBlobUrl);
      if (options.signal && abortHandler) {
        options.signal.removeEventListener("abort", abortHandler);
      }
    };

    const fail = (err) => {
      if (settled) return;
      settled = true;
      cleanup();
      reject(err instanceof Error ? err : new Error(String(err)));
    };

    const ok = (nonce) => {
      if (settled) return;
      settled = true;
      cleanup();
      resolve(nonce);
    };

    const abortHandler = () => fail(new Error("PoW solve aborted"));
    if (options.signal?.aborted) {
      return fail(new Error("PoW solve aborted"));
    }
    options.signal?.addEventListener("abort", abortHandler, { once: true });

    const prefixBase64 = bytesToBase64(prefix);

    for (let workerId = 0; workerId < workerCount; workerId++) {
      const worker = new Worker(workerBlobUrl);
      worker.onmessage = (event) => {
        if (settled) return;
        const msg = event.data || {};
        if (msg.type === "found") {
          ok(msg.nonce);
          return;
        }
        if (msg.type === "progress") {
          totalAttempts += Number(msg.attempts) || 0;
          options.onProgress?.(totalAttempts);
          return;
        }
        if (msg.type === "done") {
          doneWorkers += 1;
          if (doneWorkers === workerCount) {
            fail(
              new Error(
                `未在 ${maxIterations} 次内找到解（difficulty=${challenge.difficulty}）`
              )
            );
          }
          return;
        }
        if (msg.type === "error") {
          fail(new Error(msg.error || "PoW worker error"));
        }
      };
      worker.onerror = (event) => {
        fail(new Error(event.message || "PoW worker crashed"));
      };

      worker.postMessage({
        type: "start",
        prefixBase64,
        difficulty: challenge.difficulty,
        startNonce,
        maxIterations,
        offset: workerId,
        stride: workerCount,
        reportEvery,
      });

      workers.push(worker);
    }
  });
}

// 最小示例:
// import { solvePowNonceBrowser } from "./pow-solver-browser.js";
// const nonce = await solvePowNonceBrowser(powToken, { workers: 8 });
