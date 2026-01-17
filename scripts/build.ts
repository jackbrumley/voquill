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
  
  await verifyDependencies(false);

  const tauriDir = join(Deno.cwd(), "src-tauri");

  // Dependencies are managed by Deno via deno.json imports
  logStep("1", "Dependencies managed by Deno...");
  log(`${colors.green}✅ Using Deno's automatic dependency management${colors.reset}`);

  // 2. Build Tauri App (which will run beforeBuildCommand to build frontend)
  logStep("2", "Building Tauri application...");
  
  // Check for debug flag
  const args = Deno.args;
  const isDev = args.includes("--dev") || args.includes("-d");
  
  const tauriArgs = ["deno", "task", "tauri", "build"];
  if (isDev) {
    tauriArgs.push("--debug");
  }
  
  await runCommand(tauriArgs);
  
  log(`\n${colors.green}✅ Build completed successfully!${colors.reset}`);
}

if (import.meta.main) {
  main();
}
