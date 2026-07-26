#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { logError, run } from "@tauri-apps/cli/main.js";

if (process.platform === "win32") {
  process.env.CARGO_TARGET_DIR = "C:\\voquill-build";
}

process.env.GGML_NATIVE = "OFF";
process.env.GGML_AVX512 = "OFF";
process.env.GGML_AVX512_VBMI = "OFF";
process.env.GGML_AVX512_VNNI = "OFF";
process.env.GGML_AVX512_BF16 = "OFF";
process.env.GGML_AMX_TILE = "OFF";
process.env.GGML_AMX_INT8 = "OFF";
process.env.GGML_AMX_BF16 = "OFF";
process.env.GGML_AVX_VNNI = "OFF";

// whisper-rs-sys compiles whisper.cpp/ggml natively but does not declare
// cargo:rerun-if-env-changed for the GGML_* variables above, so changing them
// never invalidates Cargo's build cache. Without a forced rebuild, stale
// objects (e.g. compiled with /arch:AVX512 from a previous GGML_NATIVE=ON
// build) get silently relinked into the release binary. Clean the package
// before every release build so these flags always reach the compiler.
// NOTE: cargo clean -p without --release only cleans dev-profile artifacts,
// so --release is required here to match the `tauri build` profile.
if (process.argv[2] === "build") {
  execFileSync("cargo", ["clean", "-p", "whisper-rs-sys", "--release"], {
    cwd: new URL("../src-tauri", import.meta.url),
    env: process.env,
    stdio: "inherit",
  });
}

try {
  await run(process.argv.slice(2), "tauri");
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  if (typeof logError === "function") logError(message);
  console.error(error);
  process.exit(1);
}
