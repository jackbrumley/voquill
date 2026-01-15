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
  
  const dependencies = [
    { name: "libpulse-dev", apt: "libpulse-dev" },
    { name: "libgtk-layer-shell-dev", apt: "libgtk-layer-shell-dev" },
  ];

  for (const dep of dependencies) {
    const command = new Deno.Command("dpkg", {
      args: ["-s", dep.name],
      stdout: "null",
      stderr: "null",
    });

    const { success } = await command.output();
    
    if (!success) {
      console.error(`${colors.red}❌ Missing dependency: ${dep.name}${colors.reset}`);
      console.log(`${colors.yellow}Please install it with: ${colors.bright}sudo apt install ${dep.apt}${colors.reset}`);
      Deno.exit(1);
    } else {
      log(`${colors.green}✅ ${dep.name} is installed${colors.reset}`);
    }
  }
}

async function main() {
  log(`${colors.bright}${colors.magenta}🔨 Voquill Build Script (Deno)${colors.reset}`);
  
  await checkLinuxDependencies();

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
