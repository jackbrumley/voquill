#!/usr/bin/env -S deno run -A

import { exists } from "jsr:@std/fs";
import { join } from "jsr:@std/path";

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

async function checkLinuxDependencies() {
  if (Deno.build.os !== "linux") return;

  logStep("0", "Checking Linux dependencies...");
  
  // Check for libpulse-dev
  const command = new Deno.Command("dpkg", {
    args: ["-s", "libpulse-dev"],
    stdout: "null",
    stderr: "null",
  });

  const { success } = await command.output();
  
  if (!success) {
    console.error(`${colors.red}❌ Missing dependency: libpulse-dev${colors.reset}`);
    console.log(`${colors.yellow}Please install it with: ${colors.bright}sudo apt install libpulse-dev${colors.reset}`);
    Deno.exit(1);
  } else {
    log(`${colors.green}✅ libpulse-dev is installed${colors.reset}`);
  }
}

async function main() {
  log(`${colors.bright}${colors.magenta}🚀 Voquill Development Server (Deno)${colors.reset}`);
  
  // 1. Check requirements
  await checkLinuxDependencies();
  logStep("1", "Checking environment...");
  const srcDir = join(Deno.cwd(), "src");
  const uiDir = join(srcDir, "ui");
  
  if (!(await exists(uiDir))) {
    console.error(`${colors.red}❌ Could not find src/ui directory${colors.reset}`);
    Deno.exit(1);
  }

  // 2. Install Dependencies (using npm for React compatibility)
  logStep("2", "Installing frontend dependencies...");
  // We still use npm install because the vite plugins expect node_modules structure
  if (!(await exists(join(uiDir, "node_modules")))) {
    await runCommand(["npm", "install"], uiDir);
  } else {
    log(`${colors.green}✅ Dependencies already installed${colors.reset}`);
  }

  // 3. Run Dev Server
  logStep("3", "Starting Tauri dev server...");
  await runCommand(["cargo", "tauri", "dev"], srcDir);
}

if (import.meta.main) {
  main();
}
