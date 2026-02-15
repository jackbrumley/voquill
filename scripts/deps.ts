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
  apt?: string; // Debian/Ubuntu package name
  pacman?: string; // Arch/Manjaro package name
  pkgConfig?: string; // pkg-config name
  install?: {
    apt: string;
    pacman: string;
    windows?: string;
  };
  desc: string;
  defaults?: string[]; // Windows default install paths
}

const REGISTRY: Record<string, Dependency[]> = {
  linux: [
    {
      name: "libpulse",
      apt: "libpulse-dev",
      pacman: "libpulse",
      pkgConfig: "libpulse",
      install: {
        apt: "sudo apt install libpulse-dev",
        pacman: "sudo pacman -S libpulse",
      },
      desc: "PulseAudio development headers",
    },
    {
      name: "libgtk-layer-shell",
      apt: "libgtk-layer-shell-dev",
      pacman: "gtk-layer-shell",
      pkgConfig: "gtk-layer-shell-0",
      install: {
        apt: "sudo apt install libgtk-layer-shell-dev",
        pacman: "sudo pacman -S gtk-layer-shell",
      },
      desc: "GTK Layer Shell development headers",
    },
    {
      name: "cmake",
      cmd: "cmake",
      apt: "cmake",
      pacman: "cmake",
      install: {
        apt: "sudo apt install cmake",
        pacman: "sudo pacman -S cmake",
      },
      desc: "CMake build system",
    },
    {
      name: "pkg-config",
      cmd: "pkg-config",
      apt: "pkg-config",
      pacman: "pkgconf",
      install: {
        apt: "sudo apt install pkg-config",
        pacman: "sudo pacman -S pkgconf",
      },
      desc: "Package configuration tool",
    },
    {
      name: "libclang",
      apt: "libclang-dev",
      pacman: "clang",
      install: {
        apt: "sudo apt install libclang-dev",
        pacman: "sudo pacman -S clang",
      },
      desc: "Clang development headers",
    },
    {
      name: "build-essential",
      apt: "build-essential",
      pacman: "base-devel",
      cmd: "gcc",
      install: {
        apt: "sudo apt install build-essential",
        pacman: "sudo pacman -S base-devel",
      },
      desc: "Build tools (gcc, make, etc.)",
    },
    {
      name: "wl-clipboard",
      cmd: "wl-copy",
      apt: "wl-clipboard",
      pacman: "wl-clipboard",
      install: {
        apt: "sudo apt install wl-clipboard",
        pacman: "sudo pacman -S wl-clipboard",
      },
      desc: "Wayland clipboard utilities",
    },
    {
      name: "libwebkit2gtk-4.1",
      apt: "libwebkit2gtk-4.1-dev",
      pacman: "webkit2gtk-4.1",
      pkgConfig: "webkit2gtk-4.1",
      install: {
        apt: "sudo apt install libwebkit2gtk-4.1-dev",
        pacman: "sudo pacman -S webkit2gtk-4.1",
      },
      desc: "WebKitGTK development headers",
    },
    {
      name: "libgtk-3",
      apt: "libgtk-3-dev",
      pacman: "gtk3",
      pkgConfig: "gtk+-3.0",
      install: {
        apt: "sudo apt install libgtk-3-dev",
        pacman: "sudo pacman -S gtk3",
      },
      desc: "GTK3 development headers",
    },
    {
      name: "libayatana-appindicator3",
      pkgConfig: "ayatana-appindicator3-0.1",
      install: {
        apt: "sudo apt install libayatana-appindicator3-dev",
        pacman: "sudo pacman -S libayatana-appindicator",
      },
      desc: "Ayatana AppIndicator (required for Tauri tray icons)",
    },
    {
      name: "librsvg2",
      apt: "librsvg2-dev",
      pacman: "librsvg",
      pkgConfig: "librsvg-2.0",
      install: {
        apt: "sudo apt install librsvg2-dev",
        pacman: "sudo pacman -S librsvg",
      },
      desc: "librsvg development headers",
    },
    {
      name: "vulkan-headers",
      apt: "libvulkan-dev",
      pacman: "vulkan-headers",
      install: {
        apt: "sudo apt install libvulkan-dev",
        pacman: "sudo pacman -S vulkan-headers",
      },
      desc: "Vulkan development headers (required for Turbo Mode)",
    },
    {
      name: "shaderc",
      cmd: "glslc",
      apt: "glslc",
      pacman: "shaderc",
      install: {
        apt: "sudo apt install glslc",
        pacman: "sudo pacman -S shaderc",
      },
      desc: "Vulkan shader compiler (required for Turbo Mode)",
    },
    {
      name: "fuse2",
      apt: "libfuse2",
      pacman: "fuse2",
      install: {
        apt: "sudo apt install libfuse2",
        pacman: "sudo pacman -S fuse2",
      },
      desc: "FUSE 2 library (required for AppImage bundling)",
    },
  ],
  windows: [
    { 
      name: "clang", 
      cmd: "clang", 
      defaults: [
        "C:\\Program Files\\LLVM\\bin\\clang.exe",
        "C:\\Program Files (x86)\\LLVM\\bin\\clang.exe"
      ], 
      install: {
        apt: "winget install -e --id LLVM.LLVM",
        pacman: "winget install -e --id LLVM.LLVM",
        windows: "winget install -e --id LLVM.LLVM",
      }, 
      desc: "LLVM/Clang (required for bindgen)" 
    },
    { 
      name: "cmake", 
      cmd: "cmake", 
      defaults: [
        "C:\\Program Files\\CMake\\bin\\cmake.exe",
        "C:\\Program Files (x86)\\CMake\\bin\\cmake.exe"
      ], 
      install: {
        apt: "winget install -e --id Kitware.CMake",
        pacman: "winget install -e --id Kitware.CMake",
        windows: "winget install -e --id Kitware.CMake",
      }, 
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

async function checkLongPathsEnabled(): Promise<boolean> {
  try {
    const command = new Deno.Command("reg", {
      args: [
        "query",
        "HKLM\\System\\CurrentControlSet\\Control\\FileSystem",
        "/v",
        "LongPathsEnabled",
      ],
    });
    const { success, stdout } = await command.output();
    if (!success) return false;
    const output = new TextDecoder().decode(stdout);
    return output.includes("0x1");
  } catch {
    return false;
  }
}

/**
 * Ensures the Windows build artifact directory exists.
 * This short path (C:\v-target) is used to bypass MAX_PATH limits.
 */
async function checkArtifactDirectory() {
  const targetDir = "C:\\v-target";
  try {
    if (!(await exists(targetDir))) {
      console.log(
        `${colors.yellow}Creating build artifact directory: ${targetDir}...${colors.reset}`,
      );
      await Deno.mkdir(targetDir, { recursive: true });
      console.log(`${colors.green}✅ Created ${targetDir}${colors.reset}`);
    } else {
      console.log(
        `${colors.green}✅ Build artifact directory exists: ${targetDir}${colors.reset}`,
      );
    }
  } catch (error) {
    console.error(
      `${colors.red}❌ Could not prepare artifact directory: ${targetDir}${colors.reset}`,
    );
    console.log(`${colors.yellow}Error: ${error}${colors.reset}`);
    console.log(
      `${colors.yellow}This directory is required to bypass Windows path length limits during the Whisper build.${colors.reset}`,
    );
    console.log(
      `${colors.cyan}Please create the folder 'C:\\v-target' manually or run this terminal as Administrator once.${colors.reset}`,
    );
    Deno.exit(1);
  }
}


async function checkLinuxDeps(isDev: boolean) {
  console.log(`\n${colors.bright}[0]${colors.reset} ${colors.cyan}Checking Linux dependencies...${colors.reset}`);

  const hasApt = await isCommandInPath("apt-get");
  const hasPacman = await isCommandInPath("pacman");
  const hasPkgConfig = await isCommandInPath("pkg-config");

  for (const dep of REGISTRY.linux) {
    if (dep.name === "wl-clipboard" && !isDev) continue;

    let found = false;

    // 1. Check if command is in PATH
    if (dep.cmd && await isCommandInPath(dep.cmd)) {
      found = true;
    }

    // 2. Check via pkg-config (best for libraries/headers)
    if (!found && hasPkgConfig && dep.pkgConfig) {
      try {
        const process = new Deno.Command("pkg-config", {
          args: ["--exists", dep.pkgConfig],
        });
        const { success } = await process.output();
        if (success) found = true;
      } catch { /* ignore */ }
    }

    // 3. Check via package manager
    if (!found) {
      if (hasApt && dep.apt) {
        try {
          const process = new Deno.Command("dpkg", {
            args: ["-s", dep.apt],
          });
          const { success } = await process.output();
          if (success) found = true;
        } catch { /* ignore */ }
      } else if (hasPacman && dep.pacman) {
        try {
          const process = new Deno.Command("pacman", {
            args: ["-Qq", dep.pacman],
          });
          const { success } = await process.output();
          if (success) found = true;
        } catch { /* ignore */ }
      }
    }

    if (!found) {
      console.error(`${colors.red}❌ Missing dependency: ${dep.desc}${colors.reset}`);
      let installCmd = "your package manager";
      if (hasApt) installCmd = dep.install?.apt || "apt install ...";
      else if (hasPacman) installCmd = dep.install?.pacman || "pacman -S ...";
      
      console.log(`${colors.yellow}Please install it with: ${colors.bright}${installCmd}${colors.reset}`);
      Deno.exit(1);
    } else {
      console.log(`${colors.green}✅ ${dep.name} is installed${colors.reset}`);
    }
  }
}

async function checkWindowsDeps() {
  console.log(`\n${colors.bright}[0]${colors.reset} ${colors.cyan}Checking Windows build dependencies...${colors.reset}`);

  // 1. Prepare Artifact Directory (to bypass MAX_PATH issues)
  await checkArtifactDirectory();

  // 2. Check for Long Path Support (Required for LLVM/Rust toolchain)
  const longPaths = await checkLongPathsEnabled();
  if (!longPaths) {
    console.error(
      `${colors.red}❌ Windows Long Paths are NOT enabled.${colors.reset}`,
    );
    console.log(
      `${colors.yellow}This is required to build Whisper dependencies which have deep directory structures.${colors.reset}`,
    );
    console.log(
      `${colors.yellow}Please run this command in an ${colors.bright}Administrator PowerShell${colors.reset}${colors.yellow} and then restart your terminal:${colors.reset}`,
    );
    console.log(
      `${colors.bright}New-ItemProperty -Path "HKLM:\\System\\CurrentControlSet\\Control\\FileSystem" -Name "LongPathsEnabled" -Value 1 -PropertyType DWORD -Force${colors.reset}`,
    );
    Deno.exit(1);
  }
  console.log(`${colors.green}✅ Long Paths are enabled${colors.reset}`);

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
      const installCmd = dep.install?.windows || dep.install?.apt || "winget install ...";
      console.log(`${colors.yellow}Please install it with: ${colors.bright}${installCmd}${colors.reset}`);
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
