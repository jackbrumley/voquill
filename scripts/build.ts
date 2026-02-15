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
  
  const env = { ...Deno.env.toObject() };
  if (Deno.build.os === "windows") {
    // Force a short build path to avoid MAX_PATH (260 char) issues
    // Folder is verified/created in scripts/deps.ts
    env["CARGO_TARGET_DIR"] = "C:\\v-target";
  }

  const command = new Deno.Command(cmd[0], {
    args: cmd.slice(1),
    cwd,
    env,
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
  
  // Clean up any potential dev configs that might hijack the build
  const devConfigs = ["tauri.linux-dev.json", "tauri.linux.conf.json"];
  for (const config of devConfigs) {
    const configPath = join(Deno.cwd(), "src-tauri", config);
    if (await exists(configPath)) {
      log(`${colors.yellow}🧹 Cleaning up ${config}...${colors.reset}`);
      await Deno.remove(configPath);
    }
  }

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
  
  const artifactsPath = Deno.build.os === "windows" 
    ? "C:\\v-target\\release\\bundle\\" 
    : "src-tauri/target/release/bundle/";

  log(`\n${colors.green}✅ Build completed successfully!${colors.reset}`);
  log(`${colors.cyan}Artifacts can be found in: ${artifactsPath}${colors.reset}`);
}


if (import.meta.main) {
  main();
}
