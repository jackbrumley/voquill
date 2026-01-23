#!/usr/bin/env -S deno run -A

import { exists } from "jsr:@std/fs";
import { join } from "jsr:@std/path";
import { verifyDependencies } from "./deps.ts";

const colors = {
  reset: "\x1b[0m",
  bright: "\x1b[1m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  blue: "\x1b[34m",
  magenta: "\x1b[35m",
  cyan: "\x1b[36m",
  red: "\x1b[31m",
};

function log(message: string) {
  console.log(message);
}

function logStep(step: string, message: string) {
  log(`\n${colors.bright}[${step}]${colors.reset} ${colors.cyan}${message}${colors.reset}`);
}

async function runCommand(cmd: string[], cwd: string = Deno.cwd()) {
  log(`   ${colors.blue}$ ${cmd.join(" ")}${colors.reset}`);
  
  const command = new Deno.Command(cmd[0], {
    args: cmd.slice(1),
    cwd,
    stdout: "inherit",
    stderr: "inherit",
  });

  const { code } = await command.output();
  
  if (code !== 0) {
    console.error(`${colors.red}❌ Command failed with exit code ${code}${colors.reset}`);
    Deno.exit(code);
  }
}

async function main() {
  log(`${colors.bright}${colors.magenta}🔨 Voquill Build Script (Deno)${colors.reset}`);
  
  // 1. Check requirements
  await verifyDependencies(false);

  // 2. Build Tauri App
  logStep("1", "Building Tauri application...");
  
  const args = Deno.args;
  const isDebug = args.includes("--debug") || args.includes("-d");
  
  const tauriArgs = ["deno", "task", "tauri", "build"];
  
  // Ensure we DON'T use any dev configs that might be lying around
  // Tauri 2 will use tauri.conf.json by default.
  
  if (isDebug) {
    tauriArgs.push("--debug");
    log(`${colors.yellow}⚠️ Building in DEBUG mode${colors.reset}`);
  }
  
  await runCommand(tauriArgs);
  
  log(`\n${colors.green}✅ Build completed successfully!${colors.reset}`);
  log(`${colors.cyan}Artifacts can be found in: src-tauri/target/release/bundle/${colors.reset}`);
}

if (import.meta.main) {
  main();
}
