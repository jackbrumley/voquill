import { exists } from "jsr:@std/fs";

const colors = {
  reset: "\x1b[0m",
  bright: "\x1b[1m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  cyan: "\x1b[36m",
  red: "\x1b[31m",
};

interface Dependency {
  name: string;
  cmd?: string; // Command to check in PATH
  apt?: string; // Linux package name
  defaults?: string[]; // Windows default install paths
  install: string; // Installation command
  desc: string;
}

const REGISTRY: Record<string, Dependency[]> = {
  linux: [
    { name: "libpulse-dev", apt: "libpulse-dev", install: "sudo apt install libpulse-dev", desc: "PulseAudio development headers" },
    { name: "libgtk-layer-shell-dev", apt: "libgtk-layer-shell-dev", install: "sudo apt install libgtk-layer-shell-dev", desc: "GTK Layer Shell development headers" },
    { name: "cmake", cmd: "cmake", apt: "cmake", install: "sudo apt install cmake", desc: "CMake build system" },
    { name: "pkg-config", cmd: "pkg-config", apt: "pkg-config", install: "sudo apt install pkg-config", desc: "Package configuration tool" },
    { name: "libclang-dev", apt: "libclang-dev", install: "sudo apt install libclang-dev", desc: "Clang development headers" },
    { name: "build-essential", apt: "build-essential", install: "sudo apt install build-essential", desc: "Build tools (gcc, make, etc.)" },
    { name: "wl-clipboard", cmd: "wl-copy", apt: "wl-clipboard", install: "sudo apt install wl-clipboard", desc: "Wayland clipboard utilities" },
  ],
  windows: [
    { 
      name: "clang", 
      cmd: "clang", 
      defaults: [
        "C:\\Program Files\\LLVM\\bin\\clang.exe",
        "C:\\Program Files (x86)\\LLVM\\bin\\clang.exe"
      ], 
      install: "winget install -e --id LLVM.LLVM", 
      desc: "LLVM/Clang (required for bindgen)" 
    },
    { 
      name: "cmake", 
      cmd: "cmake", 
      defaults: [
        "C:\\Program Files\\CMake\\bin\\cmake.exe",
        "C:\\Program Files (x86)\\CMake\\bin\\cmake.exe"
      ], 
      install: "winget install -e --id Kitware.CMake", 
      desc: "CMake (required for building C/C++ libs)" 
    },
  ]
};

async function isCommandInPath(cmd: string): Promise<boolean> {
  try {
    const process = new Deno.Command(cmd, {
      args: ["--version"],
      stdout: "null",
      stderr: "null",
    });
    const { success } = await process.output();
    return success;
  } catch {
    return false;
  }
}

async function checkLinuxDeps(isDev: boolean) {
  console.log(`\n${colors.bright}[0]${colors.reset} ${colors.cyan}Checking Linux dependencies...${colors.reset}`);

  for (const dep of REGISTRY.linux) {
    // Skip dev-only deps if not in dev mode
    if (dep.name === "wl-clipboard" && !isDev) continue;

    // Check by dpkg first as it's the primary source of truth for headers
    const command = new Deno.Command("dpkg", {
      args: ["-s", dep.apt!],
      stdout: "null",
      stderr: "null",
    });

    const { success } = await command.output();
    if (!success) {
      console.error(`${colors.red}❌ Missing dependency: ${dep.desc}${colors.reset}`);
      console.log(`${colors.yellow}Please install it with: ${colors.bright}${dep.install}${colors.reset}`);
      Deno.exit(1);
    } else {
      console.log(`${colors.green}✅ ${dep.name} is installed${colors.reset}`);
    }
  }
}

async function checkWindowsDeps() {
  console.log(`\n${colors.bright}[0]${colors.reset} ${colors.cyan}Checking Windows build dependencies...${colors.reset}`);

  for (const dep of REGISTRY.windows) {
    // 1. Check if in PATH
    if (await isCommandInPath(dep.cmd!)) {
      console.log(`${colors.green}✅ ${dep.name} is available in PATH${colors.reset}`);
      continue;
    }

    // 2. Check default installation paths
    let foundPath = null;
    if (dep.defaults) {
      for (const path of dep.defaults) {
        if (await exists(path)) {
          foundPath = path;
          break;
        }
      }
    }

    if (foundPath) {
      const binDir = foundPath.substring(0, foundPath.lastIndexOf("\\"));
      console.error(`${colors.yellow}⚠️  ${dep.name} found but NOT in PATH${colors.reset}`);
      console.log(`${colors.cyan}Location: ${foundPath}${colors.reset}`);
      console.log(`${colors.yellow}Please add this directory to your system PATH:${colors.reset}`);
      console.log(`${colors.bright}${binDir}${colors.reset}`);
      console.log(`${colors.yellow}Then restart your terminal session.${colors.reset}`);
      Deno.exit(1);
    } else {
      console.error(`${colors.red}❌ Missing tool: ${dep.desc}${colors.reset}`);
      console.log(`${colors.yellow}Please install it with: ${colors.bright}${dep.install}${colors.reset}`);
      console.log(`${colors.yellow}Note: You MUST restart your terminal after installation.${colors.reset}`);
      Deno.exit(1);
    }
  }
}

/**
 * Verifies that all system dependencies are installed for the current platform.
 * @param isDev Whether to check for development-only dependencies.
 */
export async function verifyDependencies(isDev: boolean = false) {
  if (Deno.build.os === "linux") {
    await checkLinuxDeps(isDev);
  } else if (Deno.build.os === "windows") {
    await checkWindowsDeps();
  }
}
