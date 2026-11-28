import { spawn } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import path from "node:path";

// Starts the bundled Python runner exactly as the app does (server.py run
// directly, with the app's environment variables) and fails unless it comes
// up healthy and every endpoint reaches its capability. Run by CI on every
// platform, because nothing else starts the runner before a release.
//
// Usage: node scripts/check-python-runner.mjs [python executable]
// The Python must already have python-runner/requirements/*.txt installed.

const ROOT = process.cwd();
const RUNNER_DIR = path.join(ROOT, "python-runner");
const PYTHON = process.argv[2] ?? (process.platform === "win32" ? "python" : "python3");
const STARTUP_TIMEOUT_MS = 90_000;

/// A missing input file: each capability must be reached and reject it,
/// which proves the route is wired to a loaded module (a missing capability
/// answers 501, a missing route 404).
const MISSING_FILE = path.join(tmpdir(), "voquill-runner-check-missing.wav");
const ENDPOINT_CHECKS = [
  { path: "/diarize", body: { audio_path: MISSING_FILE, cluster_threshold: 0.5 } },
  { path: "/enhance", body: { audio_path: MISSING_FILE, noise_reduction_strength: 0.5 } },
  {
    path: "/tts/render",
    body: {
      text: "check",
      model_path: path.join(tmpdir(), "voquill-runner-check-missing.onnx"),
      tokens_path: path.join(tmpdir(), "voquill-runner-check-missing.txt"),
      data_dir: tmpdir(),
      dsp: { kind: "effect", effect: "none" },
    },
  },
];

const findFreePort = () =>
  new Promise((resolve, reject) => {
    const server = createServer();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      server.close(() => resolve(port));
    });
  });

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

const fail = (message) => {
  console.error(`Python runner check failed: ${message}`);
  process.exitCode = 1;
};

const waitForHealth = async (baseUrl, runner) => {
  const started = Date.now();
  while (Date.now() - started < STARTUP_TIMEOUT_MS) {
    if (runner.exitCode !== null) {
      throw new Error(`the runner exited with status ${runner.exitCode} before it was healthy`);
    }
    try {
      const response = await fetch(`${baseUrl}/health`);
      if (response.ok) return;
    } catch {
      // Not listening yet.
    }
    await sleep(500);
  }
  throw new Error(`the runner was not healthy within ${STARTUP_TIMEOUT_MS / 1000}s`);
};

const checkEndpoints = async (baseUrl) => {
  const capabilities = await (await fetch(`${baseUrl}/capabilities`)).json();
  console.log(`Capabilities: ${capabilities.map((capability) => capability.name).join(", ")}`);

  for (const check of ENDPOINT_CHECKS) {
    const response = await fetch(`${baseUrl}${check.path}`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify(check.body),
    });
    const detail = await response.text();
    if (response.status === 404 || response.status === 501 || response.status === 422) {
      fail(`${check.path} answered ${response.status}: ${detail}`);
    } else {
      console.log(`${check.path} reached its capability (${response.status})`);
    }
  }
};

const main = async () => {
  const port = await findFreePort();
  const baseUrl = `http://127.0.0.1:${port}`;
  const appRoot = mkdtempSync(path.join(tmpdir(), "voquill-runner-check-"));
  const runner = spawn(PYTHON, ["server.py"], {
    cwd: RUNNER_DIR,
    env: {
      ...process.env,
      VOQUILL_PORT: String(port),
      VOQUILL_PYTHON_RUNNER_DIR: RUNNER_DIR,
      VOQUILL_APP_ROOT: appRoot,
    },
    stdio: ["ignore", "inherit", "inherit"],
  });

  try {
    await waitForHealth(baseUrl, runner);
    console.log(`Runner healthy on port ${port}`);
    await checkEndpoints(baseUrl);
  } catch (error) {
    fail(error.message);
  } finally {
    runner.kill();
    rmSync(appRoot, { recursive: true, force: true });
  }
  if (!process.exitCode) {
    console.log("Python runner check passed.");
  }
};

await main();
