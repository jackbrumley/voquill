#!/usr/bin/env -S deno run -A

import { exists } from "jsr:@std/fs";
import { join } from "jsr:@std/path";
import { verifyDependencies } from "./deps.ts";

// Colors for console output
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
  log(`${colors.bright}${colors.magenta}🚀 Voquill Development Server (Deno)${colors.reset}`);
  
  // 1. Check requirements
  await verifyDependencies(true);
  logStep("1", "Checking environment...");
  const tauriDir = join(Deno.cwd(), "src-tauri");
  const srcDir = join(Deno.cwd(), "src");
  
  if (!(await exists(tauriDir))) {
    console.error(`${colors.red}❌ Could not find src-tauri directory${colors.reset}`);
    Deno.exit(1);
  }

  if (!(await exists(srcDir))) {
    console.error(`${colors.red}❌ Could not find src directory${colors.reset}`);
    Deno.exit(1);
  }

  // 2. Deno will auto-manage dependencies via nodeModulesDir: auto
  logStep("2", "Dependencies managed by Deno...");
  log(`${colors.green}✅ Using Deno's automatic dependency management${colors.reset}`);

  // 3. Run Dev Server
  logStep("3", "Starting Tauri dev server...");
  await runCommand(["deno", "task", "tauri", "dev"]);
}

if (import.meta.main) {
  main();
}
