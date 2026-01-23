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

  // 3. Register app identity for Wayland portals (Linux only)
  if (Deno.build.os === "linux") {
    logStep("3", "Registering app identity for Wayland portals...");
    try {
      const homeDir = Deno.env.get("HOME");
      if (!homeDir) {
        log(`${colors.yellow}⚠️ Could not determine HOME directory, skipping desktop file creation${colors.reset}`);
      } else {
        const applicationsDir = join(homeDir, ".local", "share", "applications");
        await Deno.mkdir(applicationsDir, { recursive: true });
        
        const desktopFilePath = join(applicationsDir, "com.voquill.voquill.desktop");
        const execPath = join(Deno.cwd(), "src-tauri", "target", "debug", "voquill");
        const iconPath = join(Deno.cwd(), "src-tauri", "icons", "icon.svg");
        
        const desktopContent = `[Desktop Entry]
Name=Voquill Dev
Comment=Voice dictation app (Development)
Exec=${execPath}
Icon=${iconPath}
Type=Application
Terminal=false
Categories=Utility;AudioVideo;
StartupWMClass=com.voquill.voquill
X-KDE-StartupNotify=false
`;
        
        await Deno.writeTextFile(desktopFilePath, desktopContent);
        log(`${colors.green}✅ Desktop file created: ${desktopFilePath}${colors.reset}`);
        
        // Update the desktop database so KDE sees it immediately
        try {
          await new Deno.Command("update-desktop-database", {
            args: [applicationsDir],
            stdout: "null",
            stderr: "null",
          }).output();
          log(`${colors.green}✅ Desktop database updated${colors.reset}`);
        } catch {
          log(`${colors.yellow}⚠️ Could not update desktop database (update-desktop-database not found)${colors.reset}`);
        }
      }
    } catch (error) {
      log(`${colors.yellow}⚠️ Failed to create desktop file: ${error}${colors.reset}`);
    }
  }

  // 4. Run Dev Server
  logStep("4", "Starting Tauri dev server...");
  await runCommand(["deno", "task", "tauri", "dev"]);
}

if (import.meta.main) {
  main();
}
