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

  // 3. Remove legacy dev desktop entry (Linux only)
  if (Deno.build.os === "linux") {
    logStep("3", "Cleaning up legacy dev desktop entry...");
    try {
      const homeDir = Deno.env.get("HOME");
      if (!homeDir) {
        log(`${colors.yellow}⚠️ Could not determine HOME directory, skipping cleanup${colors.reset}`);
      } else {
        const desktopFilePaths = [
          join(homeDir, ".local", "share", "applications", "com.voquill.voquill.desktop"),
          join(homeDir, ".local", "share", "applications", "org.voquill.foss.desktop"),
        ];

        let removedAny = false;
        for (const desktopFilePath of desktopFilePaths) {
          if (await exists(desktopFilePath)) {
            const contents = await Deno.readTextFile(desktopFilePath);
            if (contents.includes("Name=Voquill Dev")) {
              await Deno.remove(desktopFilePath);
              log(`${colors.green}✅ Removed legacy dev desktop entry: ${desktopFilePath}${colors.reset}`);
              removedAny = true;
            } else {
              log(`${colors.yellow}⚠️ Desktop file exists but is not marked as Voquill Dev; left untouched: ${desktopFilePath}${colors.reset}`);
            }
          }
        }

        if (!removedAny) {
          log(`${colors.green}✅ No legacy dev desktop entry found${colors.reset}`);
        }
      }
    } catch (error) {
      log(`${colors.yellow}⚠️ Failed to clean desktop entry: ${error}${colors.reset}`);
    }
  }

  // 4. Run Dev Server
  logStep("4", "Starting Tauri dev server...");
  const tauriArgs = ["deno", "task", "tauri", "dev"];
  
  if (Deno.build.os === "linux") {
    // 3.5 Force a rebuild of the Rust binary to ensure new commands are included
    // This is necessary because using a custom 'runner' can sometimes skip automatic recompilation
    logStep("3.5", "Building Rust binary...");
    await runCommand(["cargo", "build"], join(Deno.cwd(), "src-tauri"));

    // Dynamically generate a Linux-specific config with the absolute path to the wrapper
    // This solves the "command not found" issue in Tauri's runner while remaining portable
    const wrapperPath = join(Deno.cwd(), "src-tauri", "voquill-dev-wrapper.sh");
    const linuxConfig = {
      build: {
        runner: wrapperPath
      }
    };
    const linuxConfigPath = join(Deno.cwd(), "src-tauri", "tauri.linux-dev.json");
    await Deno.writeTextFile(linuxConfigPath, JSON.stringify(linuxConfig, null, 2));
    
    tauriArgs.push("--config", "src-tauri/tauri.linux-dev.json");
  }
  
  await runCommand(tauriArgs);
}

if (import.meta.main) {
  main();
}
