import { Dependency } from "../types.ts";
import { isCommandInPath, checkPkgConfig } from "../utils.ts";

// Helper to check for a pacman package
async function checkPacman(pkg: string): Promise<boolean> {
  try {
    const process = new Deno.Command("pacman", {
      args: ["-Qq", pkg],
      stdout: "null",
      stderr: "null",
    });
    return (await process.output()).success;
  } catch {
    return false;
  }
}

export const getArchDependencies = (): Dependency[] => [
  {
    name: "libpulse",
    desc: "PulseAudio development headers",
    check: async () => await checkPacman("libpulse"),
    install: async () => "sudo pacman -S --noconfirm libpulse",
  },
  {
    name: "libgtk-layer-shell",
    desc: "GTK Layer Shell development headers",
    check: async () => await checkPacman("gtk-layer-shell"),
    install: async () => "sudo pacman -S --noconfirm gtk-layer-shell",
  },
  {
    name: "cmake",
    desc: "CMake build system",
    check: async () => await isCommandInPath("cmake"),
    install: async () => "sudo pacman -S --noconfirm cmake",
  },
  {
    name: "pkg-config",
    desc: "Package configuration tool",
    check: async () => await isCommandInPath("pkg-config"),
    install: async () => "sudo pacman -S --noconfirm pkgconf",
  },
  {
    name: "libclang",
    desc: "Clang development headers",
    check: async () => await checkPacman("clang"),
    install: async () => "sudo pacman -S --noconfirm clang",
  },
  {
    name: "build-essential",
    desc: "Build tools (gcc, g++, make, etc.)",
    check: async () => await checkPacman("base-devel"),
    install: async () => "sudo pacman -S --noconfirm base-devel",
  },
  {
    name: "wl-clipboard",
    desc: "Wayland clipboard utilities",
    check: async () => await isCommandInPath("wl-copy"),
    install: async () => "sudo pacman -S --noconfirm wl-clipboard",
  },
  {
    name: "libwebkit2gtk-4.1",
    desc: "WebKitGTK development headers",
    check: async () => await checkPacman("webkit2gtk-4.1"),
    install: async () => "sudo pacman -S --noconfirm webkit2gtk-4.1",
  },
  {
    name: "libgtk-3",
    desc: "GTK3 development headers",
    check: async () => await checkPacman("gtk3"),
    install: async () => "sudo pacman -S --noconfirm gtk3",
  },
  {
    name: "libayatana-appindicator3",
    desc: "Ayatana AppIndicator (required for Tauri tray icons)",
    check: async () => await checkPacman("libayatana-appindicator"),
    install: async () => "sudo pacman -S --noconfirm libayatana-appindicator",
  },
  {
    name: "librsvg2",
    desc: "librsvg development headers",
    check: async () => await checkPacman("librsvg"),
    install: async () => "sudo pacman -S --noconfirm librsvg",
  },
  {
    name: "vulkan-headers",
    desc: "Vulkan development headers (required for Turbo Mode)",
    check: async () => await checkPacman("vulkan-headers"),
    install: async () => "sudo pacman -S --noconfirm vulkan-headers",
  },
  {
    name: "shaderc",
    desc: "Vulkan shader compiler (required for Turbo Mode)",
    check: async () => await checkPacman("shaderc"),
    install: async () => "sudo pacman -S --noconfirm shaderc",
  },
  {
    name: "fuse2",
    desc: "FUSE 2 library (required for AppImage bundling)",
    check: async () => await checkPacman("fuse2"),
    install: async () => "sudo pacman -S --noconfirm fuse2",
  },
  {
    name: "patchelf",
    desc: "PatchELF utility (required for AppImage bundling)",
    check: async () => await isCommandInPath("patchelf"),
    install: async () => "sudo pacman -S --noconfirm patchelf",
  },
  {
    name: "file",
    desc: "File utility (required for AppImage bundling)",
    check: async () => await isCommandInPath("file"),
    install: async () => "sudo pacman -S --noconfirm file",
  },
  {
    name: "squashfs-tools",
    desc: "SquashFS utilities (required for AppImage bundling)",
    check: async () => await isCommandInPath("mksquashfs"),
    install: async () => "sudo pacman -S --noconfirm squashfs-tools",
  },
  {
    name: "alsa",
    desc: "ALSA audio development headers",
    check: async () => await checkPacman("alsa-lib"),
    install: async () => "sudo pacman -S --noconfirm alsa-lib",
  },
  {
    name: "openssl",
    desc: "OpenSSL development headers",
    check: async () => await checkPacman("openssl"),
    install: async () => "sudo pacman -S --noconfirm openssl",
  },
  {
    name: "libudev",
    desc: "libudev development headers",
    check: async () => await checkPacman("systemd-libs"),
    install: async () => "sudo pacman -S --noconfirm systemd-libs",
  },
  {
    name: "rust",
    desc: "Rust toolchain (cargo, rustc)",
    check: async () => await isCommandInPath("cargo"),
    install: async () => "sudo pacman -S --noconfirm rustup && rustup default stable",
  },
  {
    name: "libxcrypt-compat",
    desc: "Backward compatibility for libcrypt.so.1",
    check: async () => await checkPacman("libxcrypt-compat"),
    install: async () => "sudo pacman -S --noconfirm libxcrypt-compat",
  },
  {
    name: "nodejs",
    desc: "Node.js runtime (required for UI build)",
    check: async () => await isCommandInPath("node"),
    install: async () => "sudo pacman -S --noconfirm nodejs npm",
  },
];
