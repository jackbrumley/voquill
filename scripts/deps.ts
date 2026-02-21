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
  apt?: string | string[]; // Debian/Ubuntu package name (can be an array)
  pacman?: string | string[]; // Arch/Manjaro package name (can be an array)
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
      cmd: "g++",
      install: {
        apt: "sudo apt install build-essential",
        pacman: "sudo pacman -S base-devel",
      },
      desc: "Build tools (gcc, g++, make, etc.)",
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
      apt: ["libfuse2", "libfuse2t64"],
      pacman: "fuse2",
      install: {
        apt: "sudo apt install libfuse2",
        pacman: "sudo pacman -S fuse2",
      },
      desc: "FUSE 2 library (required for AppImage bundling)",
    },
    {
      name: "patchelf",
      cmd: "patchelf",
      apt: "patchelf",
      pacman: "patchelf",
      install: {
        apt: "sudo apt install patchelf",
        pacman: "sudo pacman -S patchelf",
      },
      desc: "PatchELF utility (required for AppImage bundling)",
    },
    {
      name: "file",
      cmd: "file",
      apt: "file",
      pacman: "file",
      install: {
        apt: "sudo apt install file",
        pacman: "sudo pacman -S file",
      },
      desc: "File utility (required for AppImage bundling)",
    },
    {
      name: "alsa",
      apt: "libasound2-dev",
      pacman: "alsa-lib",
      pkgConfig: "alsa",
      install: {
        apt: "sudo apt install libasound2-dev",
        pacman: "sudo pacman -S alsa-lib",
      },
      desc: "ALSA audio development headers",
    },
    {
      name: "openssl",
      apt: "libssl-dev",
      pacman: "openssl",
      pkgConfig: "openssl",
      install: {
        apt: "sudo apt install libssl-dev",
        pacman: "sudo pacman -S openssl",
      },
      desc: "OpenSSL development headers",
    },
    {
      name: "libudev",
      apt: "libudev-dev",
      pacman: "systemd-libs",
      pkgConfig: "libudev",
      install: {
        apt: "sudo apt install libudev-dev",
        pacman: "sudo pacman -S systemd-libs",
      },
      desc: "libudev development headers",
    },
    {
      name: "rust",
      cmd: "cargo",
      desc: "Rust toolchain (cargo, rustc)",
      install: {
        apt: "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh",
        pacman: "sudo pacman -S rustup && rustup default stable",
        windows: "https://rustup.rs/",
      },
    },
    {
      name: "nodejs",
      cmd: "node",
      apt: "nodejs",
      pacman: "nodejs",
      install: {
        apt: "sudo apt install nodejs npm",
        pacman: "sudo pacman -S nodejs npm",
        windows: "winget install -e --id OpenJS.NodeJS",
      },
      desc: "Node.js runtime (required for UI build)",
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
    {
      name: "rust",
      cmd: "cargo",
      desc: "Rust toolchain (cargo, rustc)",
      install: {
        apt: "https://rustup.rs/",
        pacman: "https://rustup.rs/",
        windows: "winget install -e --id Rustlang.Rustup",
      },
    },
    {
      name: "nodejs",
      cmd: "node",
      install: {
        apt: "winget install -e --id OpenJS.NodeJS",
        pacman: "winget install -e --id OpenJS.NodeJS",
        windows: "winget install -e --id OpenJS.NodeJS",
      },
      desc: "Node.js runtime (required for UI build)",
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

  const missingPackages: string[] = [];
  const missingDescriptions: string[] = [];

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
        const aptPackages = Array.isArray(dep.apt) ? dep.apt : [dep.apt];
        for (const pkg of aptPackages) {
          try {
            const process = new Deno.Command("dpkg", {
              args: ["-s", pkg],
            });
            const { success } = await process.output();
            if (success) {
              found = true;
              break;
            }
          } catch { /* ignore */ }
        }
      } else if (hasPacman && dep.pacman) {
        const pacmanPackages = Array.isArray(dep.pacman) ? dep.pacman : [dep.pacman];
        for (const pkg of pacmanPackages) {
          try {
            const process = new Deno.Command("pacman", {
              args: ["-Qq", pkg],
            });
            const { success } = await process.output();
            if (success) {
              found = true;
              break;
            }
          } catch { /* ignore */ }
        }
      }
    }

    if (!found) {
      console.error(`${colors.red}❌ Missing: ${dep.name}${colors.reset} (${dep.desc})`);
      missingDescriptions.push(`${dep.name} (${dep.desc})`);
      
      if (hasApt && dep.apt) {
        const pkg = Array.isArray(dep.apt) ? dep.apt[0] : dep.apt;
        missingPackages.push(pkg);
      } else if (hasPacman && dep.pacman) {
        const pkg = Array.isArray(dep.pacman) ? dep.pacman[0] : dep.pacman;
        missingPackages.push(pkg);
      } else if (dep.install) {
        // Handle tools with custom install commands (like Rust)
        const customCmd = hasApt ? dep.install.apt : (hasPacman ? dep.install.pacman : "");
        if (customCmd) {
          missingDescriptions.push(`${colors.yellow}👉 To install ${dep.name}: ${colors.bright}${customCmd}${colors.reset}`);
        }
      }
    } else {
      console.log(`${colors.green}✅ ${dep.name} is installed${colors.reset}`);
    }
  }

  if (missingPackages.length > 0 || missingDescriptions.some(d => d.includes("👉"))) {
    if (missingPackages.length > 0) {
      console.log(`\n${colors.bright}${colors.red}Found ${missingPackages.length} missing system packages!${colors.reset}`);
      
      let installCmd = "";
      if (hasApt) {
        installCmd = `sudo apt install ${missingPackages.join(" ")}`;
      } else if (hasPacman) {
        installCmd = `sudo pacman -S ${missingPackages.join(" ")}`;
      }

      if (installCmd) {
        console.log(`${colors.yellow}Please install them with:${colors.reset}`);
        console.log(`${colors.bright}${colors.cyan}${installCmd}${colors.reset}`);
      }
    }

    const specialInstalls = missingDescriptions.filter(d => d.includes("👉"));
    if (specialInstalls.length > 0) {
      console.log(`\n${colors.bright}${colors.yellow}Additional setup required:${colors.reset}`);
      for (const special of specialInstalls) {
        console.log(special);
      }
    }
    
    Deno.exit(1);
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

  const missingTools: string[] = [];
  const toolsNotInPath: { name: string; path: string; binDir: string }[] = [];

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
      toolsNotInPath.push({ name: dep.name, path: foundPath, binDir });
      console.error(`${colors.yellow}⚠️  ${dep.name} found but NOT in PATH${colors.reset}`);
    } else {
      missingTools.push(dep.name);
      console.error(`${colors.red}❌ Missing tool: ${dep.name} (${dep.desc})${colors.reset}`);
    }
  }

  if (missingTools.length > 0 || toolsNotInPath.length > 0) {
    if (missingTools.length > 0) {
      const installIds = missingTools.map(name => {
        const dep = REGISTRY.windows.find(d => d.name === name);
        return dep?.install?.windows?.split(" ").pop() || name;
      });

      console.log(`\n${colors.yellow}Please install the missing tools with:${colors.reset}`);
      console.log(`${colors.bright}${colors.cyan}winget install ${installIds.join(" ")}${colors.reset}`);
      console.log(`${colors.yellow}Note: You MUST restart your terminal after installation.${colors.reset}`);
    }

    if (toolsNotInPath.length > 0) {
      console.log(`\n${colors.yellow}The following tools are installed but not in your PATH:${colors.reset}`);
      for (const tool of toolsNotInPath) {
        console.log(`${colors.cyan}- ${tool.name}: ${tool.path}${colors.reset}`);
        console.log(`  Add this to PATH: ${colors.bright}${tool.binDir}${colors.reset}`);
      }
      console.log(`${colors.yellow}Then restart your terminal session.${colors.reset}`);
    }
    
    Deno.exit(1);
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
