#!/usr/bin/env node

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

try {
  await run(process.argv.slice(2), "tauri");
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  if (typeof logError === "function") logError(message);
  console.error(error);
  process.exit(1);
}
