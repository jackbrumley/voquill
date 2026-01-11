#!/usr/bin/env -S deno run -A

import { exists } from "jsr:@std/fs";
import { join } from "jsr:@std/path";

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
  log(`${colors.bright}${colors.magenta}🔨 Voquill Build Script (Deno)${colors.reset}`);
  
  await checkLinuxDependencies();

  const srcDir = join(Deno.cwd(), "src");
  const uiDir = join(srcDir, "ui");

  // 1. Install Dependencies
  logStep("1", "Installing frontend dependencies...");
  await runCommand(["npm", "install"], uiDir);

  // 2. Build Frontend
  logStep("2", "Building frontend...");
  await runCommand(["npm", "run", "build"], uiDir);

  // 3. Build Tauri App
  logStep("3", "Building Tauri application...");
  
  // Check for debug flag
  const args = Deno.args;
  const isDev = args.includes("--dev") || args.includes("-d");
  
  const tauriArgs = ["cargo", "tauri", "build"];
  if (isDev) {
    tauriArgs.push("--debug");
  }
  
  await runCommand(tauriArgs, srcDir);
  
  log(`\n${colors.green}✅ Build completed successfully!${colors.reset}`);
}

if (import.meta.main) {
  main();
}
