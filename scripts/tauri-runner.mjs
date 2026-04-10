#!/usr/bin/env node

import { logError, run } from "@tauri-apps/cli/main.js";

if (process.platform === "win32") {
  process.env.CARGO_TARGET_DIR = "C:\\voquill-build";
}

try {
  await run(process.argv.slice(2), "tauri");
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  if (typeof logError === "function") logError(message);
  console.error(error);
  process.exit(1);
}
