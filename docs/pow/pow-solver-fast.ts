import { createHash } from "node:crypto";
import { cpus } from "node:os";
import { Worker } from "node:worker_threads";

export interface PowChallenge {
  v: number;
  action: string;
  target_type: string;
  target_id: number;
  user_id: number;
  issued_at: number;
  expires_at: number;
  difficulty: number;
  salt: string;
}

export interface SolvePowFastOptions {
  // 并行 worker 数，默认 max(1, CPU核数 - 1)
  workers?: number;
  // nonce 起始值（通常从 0 开始）
  startNonce?: number;
  // 总尝试次数上限（全体 worker 合计）
  maxIterations?: number;
  // 进度回调（attempts 为全体累计尝试次数）
  onProgress?: (attempts: number) => void;
  // worker 上报频率（每 worker 每 N 次）
  reportEvery?: number;
  // 可选取消信号
  signal?: AbortSignal;
}

type WorkerMessage =
  | { type: "progress"; attempts: number }
  | { type: "found"; nonce: string }
  | { type: "done" }
  | { type: "error"; error: string };

function base64UrlToBuffer(input: string): Buffer {
  const base64 = input.replace(/-/g, "+").replace(/_/g, "/");
  const padded = base64.padEnd(Math.ceil(base64.length / 4) * 4, "=");
  return Buffer.from(padded, "base64");
}

function int32Le(value: number): Buffer {
  const b = Buffer.allocUnsafe(4);
  b.writeInt32LE(value, 0);
  return b;
}

function int64Le(value: number): Buffer {
  const b = Buffer.allocUnsafe(8);
  b.writeBigInt64LE(BigInt(Math.trunc(value)), 0);
  return b;
}

function hasLeadingZeroBits(bytes: Uint8Array, bits: number): boolean {
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

function buildPowPrefix(ch: PowChallenge): Buffer {
  const sep = Buffer.from("|");
  return Buffer.concat([
    Buffer.from(ch.action, "utf8"),
    sep,
    Buffer.from(ch.target_type, "utf8"),
    sep,
    int32Le(ch.target_id),
    sep,
    int32Le(ch.user_id),
    sep,
    int64Le(ch.issued_at),
    sep,
    int64Le(ch.expires_at),
    sep,
    Buffer.from([ch.difficulty & 0xff]),
    sep,
    Buffer.from(ch.salt, "utf8"),
    sep,
  ]);
}

function validateChallengeShape(raw: Record<string, unknown>): PowChallenge {
  const mustNum = (k: string): number => {
    const v = raw[k];
    if (typeof v !== "number" || !Number.isFinite(v)) {
      throw new Error(`pow_token payload 字段无效: ${k}`);
    }
    return v;
  };
  const mustStr = (k: string): string => {
    const v = raw[k];
    if (typeof v !== "string") {
      throw new Error(`pow_token payload 字段无效: ${k}`);
    }
    return v;
  };

  return {
    v: mustNum("v"),
    action: mustStr("action"),
    target_type: mustStr("target_type"),
    target_id: mustNum("target_id"),
    user_id: mustNum("user_id"),
    issued_at: mustNum("issued_at"),
    expires_at: mustNum("expires_at"),
    difficulty: mustNum("difficulty"),
    salt: mustStr("salt"),
  };
}

export function decodePowChallenge(powToken: string): PowChallenge {
  const [payloadB64] = powToken.split(".");
  if (!payloadB64) {
    throw new Error("Invalid pow_token");
  }
  const payload = base64UrlToBuffer(payloadB64).toString("utf8");
  return validateChallengeShape(JSON.parse(payload) as Record<string, unknown>);
}

function solveSingleThread(
  prefix: Buffer,
  difficulty: number,
  startNonce: number,
  maxIterations: number,
  reportEvery: number,
  onProgress?: (attempts: number) => void,
  signal?: AbortSignal
): string {
  let attempts = 0;
  for (let i = 0; i < maxIterations; i++) {
    if (signal?.aborted) {
      throw new Error("PoW solve aborted");
    }

    const nonce = String(startNonce + i);
    const digest = createHash("sha256")
      .update(prefix)
      .update(nonce, "utf8")
      .digest();
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

function makeAbortError(): Error {
  return new Error("PoW solve aborted");
}

const WORKER_SOURCE = `
const { parentPort, workerData } = require("node:worker_threads");
const { createHash } = require("node:crypto");

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

try {
  const {
    prefixBase64,
    difficulty,
    startNonce,
    maxIterations,
    offset,
    stride,
    reportEvery,
  } = workerData;

  const prefix = Buffer.from(prefixBase64, "base64");
  let localAttempts = 0;

  for (let i = offset; i < maxIterations; i += stride) {
    const nonce = String(startNonce + i);
    const digest = createHash("sha256")
      .update(prefix)
      .update(nonce, "utf8")
      .digest();
    localAttempts++;

    if (hasLeadingZeroBits(digest, difficulty)) {
      parentPort.postMessage({ type: "found", nonce });
      return;
    }

    if (localAttempts % reportEvery === 0) {
      parentPort.postMessage({ type: "progress", attempts: reportEvery });
    }
  }

  const rem = localAttempts % reportEvery;
  if (rem > 0) {
    parentPort.postMessage({ type: "progress", attempts: rem });
  }
  parentPort.postMessage({ type: "done" });
} catch (e) {
  const msg = e && e.message ? e.message : String(e);
  parentPort.postMessage({ type: "error", error: msg });
}
`;

export async function solvePowNonceFast(
  powToken: string,
  options: SolvePowFastOptions = {}
): Promise<string> {
  const challenge = decodePowChallenge(powToken);
  const prefix = buildPowPrefix(challenge);

  const cpuBasedDefault = Math.max(1, cpus().length - 1);
  const workerCountRaw = options.workers ?? cpuBasedDefault;
  const workerCount = Math.max(1, Math.floor(workerCountRaw));
  const startNonce = Math.trunc(options.startNonce ?? 0);
  const maxIterations = Math.max(
    1,
    Math.trunc(options.maxIterations ?? 50_000_000)
  );
  const reportEvery = Math.max(1, Math.trunc(options.reportEvery ?? 50_000));

  if (workerCount === 1) {
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

  return new Promise<string>((resolve, reject) => {
    let settled = false;
    let totalAttempts = 0;
    let doneWorkers = 0;
    const workers: Worker[] = [];

    const cleanup = () => {
      for (const w of workers) {
        void w.terminate();
      }
      if (options.signal && abortHandler) {
        options.signal.removeEventListener("abort", abortHandler);
      }
    };

    const fail = (err: Error) => {
      if (settled) return;
      settled = true;
      cleanup();
      reject(err);
    };

    const ok = (nonce: string) => {
      if (settled) return;
      settled = true;
      cleanup();
      resolve(nonce);
    };

    const abortHandler = () => fail(makeAbortError());
    if (options.signal?.aborted) {
      return fail(makeAbortError());
    }
    options.signal?.addEventListener("abort", abortHandler, { once: true });

    const prefixBase64 = prefix.toString("base64");

    for (let workerId = 0; workerId < workerCount; workerId++) {
      const worker = new Worker(WORKER_SOURCE, {
        eval: true,
        workerData: {
          prefixBase64,
          difficulty: challenge.difficulty,
          startNonce,
          maxIterations,
          offset: workerId,
          stride: workerCount,
          reportEvery,
        },
      });

      worker.on("message", (msg: WorkerMessage) => {
        if (settled) return;
        if (msg.type === "found") {
          ok(msg.nonce);
          return;
        }
        if (msg.type === "progress") {
          totalAttempts += msg.attempts;
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
          fail(new Error(msg.error));
        }
      });

      worker.on("error", (e) => fail(e));
      worker.on("exit", (code) => {
        if (!settled && code !== 0) {
          fail(new Error(`PoW worker exited with code ${code}`));
        }
      });

      workers.push(worker);
    }
  });
}

// 最小示例:
// const nonce = await solvePowNonceFast(powToken, { workers: 8 });
// 请求投票时附带: { pow_token: powToken, pow_nonce: nonce, value: 1 }
