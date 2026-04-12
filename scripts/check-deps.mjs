#!/usr/bin/env node

import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";

const colors = {
  reset: "\x1b[0m",
  bright: "\x1b[1m",
  green: "\x1b[32m",
  yellow: "\x1b[33m",
  cyan: "\x1b[36m",
  red: "\x1b[31m",
};

function runSuccess(command, args) {
  try {
    const result = spawnSync(command, args, { stdio: "ignore" });
    return result.status === 0;
  } catch {
    return false;
  }
}

function commandExists(command) {
  const locator = process.platform === "win32" ? "where.exe" : "which";
  if (runSuccess(locator, [command])) {
    return true;
  }

  if (command === "cargo") {
    const cargoPath = process.platform === "win32"
      ? path.join(os.homedir(), ".cargo", "bin", "cargo.exe")
      : path.join(os.homedir(), ".cargo", "bin", "cargo");
    return fs.existsSync(cargoPath);
  }

  return false;
}

function checkPkgConfig(pkg) {
  return commandExists("pkg-config") && runSuccess("pkg-config", ["--exists", pkg]);
}

function checkDeb(pkg) {
  return runSuccess("dpkg", ["-s", pkg]);
}

function checkRpm(pkg) {
  return runSuccess("rpm", ["-q", pkg]);
}

function checkPacman(pkg) {
  return runSuccess("pacman", ["-Qq", pkg]);
}

function getWindowsDependencies() {
  const userProfile = process.env.USERPROFILE ?? "";
  const programFiles = process.env.ProgramFiles ?? "C:\\Program Files";
  const programFilesX86 = process.env["ProgramFiles(x86)"] ?? "C:\\Program Files (x86)";

  return [
    {
      name: "clang",
      desc: "LLVM/Clang (required for bindgen)",
      check: () => commandExists("clang") || fs.existsSync("C:\\Program Files\\LLVM\\bin\\clang.exe") ||
        fs.existsSync("C:\\Program Files (x86)\\LLVM\\bin\\clang.exe"),
      install: "winget install -e --id LLVM.LLVM",
    },
    {
      name: "cmake",
      desc: "CMake (required for building C/C++ libs)",
      check: () => commandExists("cmake") || fs.existsSync("C:\\Program Files\\CMake\\bin\\cmake.exe") ||
        fs.existsSync("C:\\Program Files (x86)\\CMake\\bin\\cmake.exe"),
      install: "winget install -e --id Kitware.CMake",
    },
    {
      name: "rust",
      desc: "Rust toolchain (cargo, rustc)",
      check: () => commandExists("cargo") || (userProfile ? fs.existsSync(path.join(userProfile, ".cargo", "bin", "cargo.exe")) : false),
      install: "winget install -e --id Rustlang.Rustup",
    },
    {
      name: "nodejs",
      desc: "Node.js runtime (required for UI build)",
      check: () => commandExists("node") || fs.existsSync(path.join(programFiles, "nodejs", "node.exe")) ||
        fs.existsSync(path.join(programFilesX86, "nodejs", "node.exe")),
      install: "winget install -e --id OpenJS.NodeJS",
    },
  ];
}

function getFedoraDependencies() {
  return [
    ["libpulse", "PulseAudio development headers", () => checkPkgConfig("libpulse"), "sudo dnf install -y pulseaudio-libs-devel"],
    ["libgtk-layer-shell", "GTK Layer Shell development headers", () => checkPkgConfig("gtk-layer-shell-0"), "sudo dnf install -y gtk-layer-shell-devel"],
    ["cmake", "CMake build system", () => commandExists("cmake"), "sudo dnf install -y cmake"],
    ["pkg-config", "Package configuration tool", () => commandExists("pkg-config"), "sudo dnf install -y pkgconf-pkg-config"],
    ["libclang", "Clang development headers", () => checkRpm("clang-devel"), "sudo dnf install -y clang-devel"],
    ["build-essential", "Build tools (gcc, g++, make, etc.)", () => checkRpm("gcc-c++") && commandExists("make"), "sudo dnf install -y gcc-c++ make"],
    ["glibc-headers", "C Standard Library development headers", () => checkRpm("glibc-devel"), "sudo dnf install -y glibc-devel"],
    ["wl-clipboard", "Wayland clipboard utilities", () => commandExists("wl-copy"), "sudo dnf install -y wl-clipboard"],
    ["libwebkit2gtk-4.1", "WebKitGTK development headers", () => checkPkgConfig("webkit2gtk-4.1"), "sudo dnf install -y webkit2gtk4.1-devel"],
    ["libgtk-3", "GTK3 development headers", () => checkPkgConfig("gtk+-3.0"), "sudo dnf install -y gtk3-devel"],
    ["libayatana-appindicator3", "Ayatana AppIndicator (required for Tauri tray icons)", () => checkPkgConfig("ayatana-appindicator3-0.1"), "sudo dnf install -y libayatana-appindicator-gtk3-devel"],
    ["librsvg2", "librsvg development headers", () => checkPkgConfig("librsvg-2.0"), "sudo dnf install -y librsvg2-devel"],
    ["vulkan-headers", "Vulkan development headers (required for Turbo Mode)", () => checkRpm("vulkan-headers"), "sudo dnf install -y vulkan-headers vulkan-loader-devel"],
    ["shaderc", "Vulkan shader compiler (required for Turbo Mode)", () => commandExists("glslc"), "sudo dnf install -y glslc"],
    ["fuse2", "FUSE 2 library (required for AppImage bundling)", () => checkRpm("fuse-libs"), "sudo dnf install -y fuse-libs"],
    ["patchelf", "PatchELF utility (required for AppImage bundling)", () => commandExists("patchelf"), "sudo dnf install -y patchelf"],
    ["file", "File utility (required for AppImage bundling)", () => commandExists("file"), "sudo dnf install -y file"],
    ["squashfs-tools", "SquashFS utilities (required for AppImage bundling)", () => commandExists("mksquashfs"), "sudo dnf install -y squashfs-tools"],
    ["appstream", "AppStream CLI (required for AppImage bundling)", () => commandExists("appstreamcli"), "sudo dnf install -y appstream"],
    ["desktop-file-utils", "Desktop file validator (required for AppImage bundling)", () => commandExists("desktop-file-validate"), "sudo dnf install -y desktop-file-utils"],
    ["alsa", "ALSA audio development headers", () => checkPkgConfig("alsa"), "sudo dnf install -y alsa-lib-devel"],
    ["openssl", "OpenSSL development headers", () => checkPkgConfig("openssl"), "sudo dnf install -y openssl-devel"],
    ["libudev", "libudev development headers", () => checkPkgConfig("libudev"), "sudo dnf install -y systemd-devel"],
    ["rust", "Rust toolchain (cargo, rustc)", () => commandExists("cargo"), "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && source $HOME/.cargo/env"],
    ["libxcrypt-compat", "Backward compatibility for libcrypt.so.1", () => checkRpm("libxcrypt-compat"), "sudo dnf install -y libxcrypt-compat"],
    ["nodejs", "Node.js runtime (required for UI build)", () => commandExists("node"), "sudo dnf install -y nodejs npm"],
  ].map(([name, desc, check, install]) => ({ name, desc, check, install }));
}

function getDebianDependencies() {
  return [
    ["libpulse", "PulseAudio development headers", () => checkDeb("libpulse-dev"), "sudo apt install -y libpulse-dev"],
    ["libgtk-layer-shell", "GTK Layer Shell development headers", () => checkDeb("libgtk-layer-shell-dev"), "sudo apt install -y libgtk-layer-shell-dev"],
    ["cmake", "CMake build system", () => commandExists("cmake"), "sudo apt install -y cmake"],
    ["pkg-config", "Package configuration tool", () => commandExists("pkg-config"), "sudo apt install -y pkg-config"],
    ["libclang", "Clang development headers", () => checkDeb("libclang-dev"), "sudo apt install -y libclang-dev"],
    ["build-essential", "Build tools (gcc, g++, make, etc.)", () => commandExists("g++"), "sudo apt install -y build-essential"],
    ["wl-clipboard", "Wayland clipboard utilities", () => commandExists("wl-copy"), "sudo apt install -y wl-clipboard"],
    ["libwebkit2gtk-4.1", "WebKitGTK development headers", () => checkDeb("libwebkit2gtk-4.1-dev"), "sudo apt install -y libwebkit2gtk-4.1-dev"],
    ["libgtk-3", "GTK3 development headers", () => checkDeb("libgtk-3-dev"), "sudo apt install -y libgtk-3-dev"],
    ["libayatana-appindicator3", "Ayatana AppIndicator (required for Tauri tray icons)", () => checkDeb("libayatana-appindicator3-dev"), "sudo apt install -y libayatana-appindicator3-dev"],
    ["librsvg2", "librsvg development headers", () => checkDeb("librsvg2-dev"), "sudo apt install -y librsvg2-dev"],
    ["vulkan-headers", "Vulkan development headers (required for Turbo Mode)", () => checkDeb("libvulkan-dev"), "sudo apt install -y libvulkan-dev"],
    ["shaderc", "Vulkan shader compiler (required for Turbo Mode)", () => commandExists("glslc"), "sudo apt install -y glslc"],
    [
      "fuse2",
      "FUSE 2 library (required for AppImage bundling)",
      () => checkDeb("libfuse2") || checkDeb("libfuse2t64"),
      "sudo apt install -y libfuse2 || sudo apt install -y libfuse2t64",
    ],
    ["patchelf", "PatchELF utility (required for AppImage bundling)", () => commandExists("patchelf"), "sudo apt install -y patchelf"],
    ["file", "File utility (required for AppImage bundling)", () => commandExists("file"), "sudo apt install -y file"],
    ["squashfs-tools", "SquashFS utilities (required for AppImage bundling)", () => commandExists("mksquashfs"), "sudo apt install -y squashfs-tools"],
    ["appstream", "AppStream CLI (required for AppImage bundling)", () => commandExists("appstreamcli"), "sudo apt install -y appstream"],
    ["desktop-file-utils", "Desktop file validator (required for AppImage bundling)", () => commandExists("desktop-file-validate"), "sudo apt install -y desktop-file-utils"],
    ["alsa", "ALSA audio development headers", () => checkDeb("libasound2-dev"), "sudo apt install -y libasound2-dev"],
    ["openssl", "OpenSSL development headers", () => checkDeb("libssl-dev"), "sudo apt install -y libssl-dev"],
    ["libudev", "libudev development headers", () => checkDeb("libudev-dev"), "sudo apt install -y libudev-dev"],
    ["rust", "Rust toolchain (cargo, rustc)", () => commandExists("cargo"), "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"],
    ["nodejs", "Node.js runtime (required for UI build)", () => commandExists("node"), "sudo apt install -y nodejs npm"],
  ].map(([name, desc, check, install]) => ({ name, desc, check, install }));
}

function getArchDependencies() {
  return [
    ["libpulse", "PulseAudio development headers", () => checkPacman("libpulse"), "sudo pacman -S --noconfirm libpulse"],
    ["libgtk-layer-shell", "GTK Layer Shell development headers", () => checkPacman("gtk-layer-shell"), "sudo pacman -S --noconfirm gtk-layer-shell"],
    ["cmake", "CMake build system", () => commandExists("cmake"), "sudo pacman -S --noconfirm cmake"],
    ["pkg-config", "Package configuration tool", () => commandExists("pkg-config"), "sudo pacman -S --noconfirm pkgconf"],
    ["libclang", "Clang development headers", () => checkPacman("clang"), "sudo pacman -S --noconfirm clang"],
    ["build-essential", "Build tools (gcc, g++, make, etc.)", () => checkPacman("base-devel"), "sudo pacman -S --noconfirm base-devel"],
    ["wl-clipboard", "Wayland clipboard utilities", () => commandExists("wl-copy"), "sudo pacman -S --noconfirm wl-clipboard"],
    ["libwebkit2gtk-4.1", "WebKitGTK development headers", () => checkPacman("webkit2gtk-4.1"), "sudo pacman -S --noconfirm webkit2gtk-4.1"],
    ["libgtk-3", "GTK3 development headers", () => checkPacman("gtk3"), "sudo pacman -S --noconfirm gtk3"],
    ["libayatana-appindicator3", "Ayatana AppIndicator (required for Tauri tray icons)", () => checkPacman("libayatana-appindicator"), "sudo pacman -S --noconfirm libayatana-appindicator"],
    ["librsvg2", "librsvg development headers", () => checkPacman("librsvg"), "sudo pacman -S --noconfirm librsvg"],
    ["vulkan-headers", "Vulkan development headers (required for Turbo Mode)", () => checkPacman("vulkan-headers"), "sudo pacman -S --noconfirm vulkan-headers"],
    ["shaderc", "Vulkan shader compiler (required for Turbo Mode)", () => checkPacman("shaderc"), "sudo pacman -S --noconfirm shaderc"],
    ["fuse2", "FUSE 2 library (required for AppImage bundling)", () => checkPacman("fuse2"), "sudo pacman -S --noconfirm fuse2"],
    ["patchelf", "PatchELF utility (required for AppImage bundling)", () => commandExists("patchelf"), "sudo pacman -S --noconfirm patchelf"],
    ["file", "File utility (required for AppImage bundling)", () => commandExists("file"), "sudo pacman -S --noconfirm file"],
    ["squashfs-tools", "SquashFS utilities (required for AppImage bundling)", () => commandExists("mksquashfs"), "sudo pacman -S --noconfirm squashfs-tools"],
    ["appstream", "AppStream CLI (required for AppImage bundling)", () => commandExists("appstreamcli"), "sudo pacman -S --noconfirm appstream"],
    ["desktop-file-utils", "Desktop file validator (required for AppImage bundling)", () => commandExists("desktop-file-validate"), "sudo pacman -S --noconfirm desktop-file-utils"],
    ["alsa", "ALSA audio development headers", () => checkPacman("alsa-lib"), "sudo pacman -S --noconfirm alsa-lib"],
    ["openssl", "OpenSSL development headers", () => checkPacman("openssl"), "sudo pacman -S --noconfirm openssl"],
    ["libudev", "libudev development headers", () => checkPacman("systemd-libs"), "sudo pacman -S --noconfirm systemd-libs"],
    ["rust", "Rust toolchain (cargo, rustc)", () => commandExists("cargo"), "sudo pacman -S --noconfirm rustup && rustup default stable"],
    ["libxcrypt-compat", "Backward compatibility for libcrypt.so.1", () => checkPacman("libxcrypt-compat"), "sudo pacman -S --noconfirm libxcrypt-compat"],
    ["nodejs", "Node.js runtime (required for UI build)", () => commandExists("node"), "sudo pacman -S --noconfirm nodejs npm"],
  ].map(([name, desc, check, install]) => ({ name, desc, check, install }));
}

function getDependenciesForCurrentSystem() {
  if (process.platform === "win32") {
    console.log(`${colors.cyan}Detected Windows system${colors.reset}`);
    return getWindowsDependencies();
  }

  if (process.platform !== "linux") {
    console.error(`${colors.red}Unsupported OS: ${process.platform}${colors.reset}`);
    process.exit(1);
  }

  const osReleasePath = "/etc/os-release";
  let release = "";
  if (fs.existsSync(osReleasePath)) {
    release = fs.readFileSync(osReleasePath, "utf8").toLowerCase();
  }

  if (release.includes("fedora") || release.includes("rhel")) {
    console.log(`${colors.cyan}Detected Fedora/RHEL-based system${colors.reset}`);
    return getFedoraDependencies();
  }

  if (release.includes("arch")) {
    console.log(`${colors.cyan}Detected Arch-based system${colors.reset}`);
    return getArchDependencies();
  }

  console.log(`${colors.cyan}Detected Debian/Ubuntu-based system${colors.reset}`);
  return getDebianDependencies();
}

function main() {
  console.log(`\n${colors.bright}[0]${colors.reset} ${colors.cyan}Checking system dependencies...${colors.reset}`);
  const dependencies = getDependenciesForCurrentSystem();
  const missing = [];

  for (const dep of dependencies) {
    if (dep.check()) {
      console.log(`${colors.green}✅ ${dep.name} is installed${colors.reset}`);
    } else {
      console.error(`${colors.red}❌ Missing: ${dep.name}${colors.reset} (${dep.desc})`);
      missing.push(dep);
    }
  }

  if (missing.length > 0) {
    console.log(`\n${colors.bright}${colors.red}Found ${missing.length} missing system dependencies!${colors.reset}`);
    console.log(`${colors.yellow}Please run the following commands to install them:${colors.reset}`);
    const installCommands = [...new Set(missing.map((dep) => dep.install))];
    for (const command of installCommands) {
      console.log(`${colors.bright}${colors.cyan}${command}${colors.reset}`);
    }
    process.exit(1);
  }

  console.log(`\n${colors.green}✅ All system dependencies are installed!${colors.reset}`);
}

main();
