import { Dependency } from "../types.ts";
import { isCommandInPath } from "../utils.ts";

// Helper to check for a debian package
async function checkApt(pkg: string): Promise<boolean> {
  try {
    const process = new Deno.Command("dpkg", {
      args: ["-s", pkg],
      stdout: "null",
      stderr: "null",
    });
    return (await process.output()).success;
  } catch {
    return false;
  }
}

export const getDebianDependencies = (): Dependency[] => [
  {
    name: "libpulse",
    desc: "PulseAudio development headers",
    check: async () => await checkApt("libpulse-dev"),
    install: async () => "sudo apt install -y libpulse-dev",
  },
  {
    name: "libgtk-layer-shell",
    desc: "GTK Layer Shell development headers",
    check: async () => await checkApt("libgtk-layer-shell-dev"),
    install: async () => "sudo apt install -y libgtk-layer-shell-dev",
  },
  {
    name: "cmake",
    desc: "CMake build system",
    check: async () => await isCommandInPath("cmake"),
    install: async () => "sudo apt install -y cmake",
  },
  {
    name: "pkg-config",
    desc: "Package configuration tool",
    check: async () => await isCommandInPath("pkg-config"),
    install: async () => "sudo apt install -y pkg-config",
  },
  {
    name: "libclang",
    desc: "Clang development headers",
    check: async () => await checkApt("libclang-dev"),
    install: async () => "sudo apt install -y libclang-dev",
  },
  {
    name: "build-essential",
    desc: "Build tools (gcc, g++, make, etc.)",
    check: async () => await isCommandInPath("g++"),
    install: async () => "sudo apt install -y build-essential",
  },
  {
    name: "wl-clipboard",
    desc: "Wayland clipboard utilities",
    check: async () => await isCommandInPath("wl-copy"),
    install: async () => "sudo apt install -y wl-clipboard",
  },
  {
    name: "libwebkit2gtk-4.1",
    desc: "WebKitGTK development headers",
    check: async () => await checkApt("libwebkit2gtk-4.1-dev"),
    install: async () => "sudo apt install -y libwebkit2gtk-4.1-dev",
  },
  {
    name: "libgtk-3",
    desc: "GTK3 development headers",
    check: async () => await checkApt("libgtk-3-dev"),
    install: async () => "sudo apt install -y libgtk-3-dev",
  },
  {
    name: "libayatana-appindicator3",
    desc: "Ayatana AppIndicator (required for Tauri tray icons)",
    check: async () => await checkApt("libayatana-appindicator3-dev"),
    install: async () => "sudo apt install -y libayatana-appindicator3-dev",
  },
  {
    name: "librsvg2",
    desc: "librsvg development headers",
    check: async () => await checkApt("librsvg2-dev"),
    install: async () => "sudo apt install -y librsvg2-dev",
  },
  {
    name: "vulkan-headers",
    desc: "Vulkan development headers (required for Turbo Mode)",
    check: async () => await checkApt("libvulkan-dev"),
    install: async () => "sudo apt install -y libvulkan-dev",
  },
  {
    name: "shaderc",
    desc: "Vulkan shader compiler (required for Turbo Mode)",
    check: async () => await isCommandInPath("glslc"),
    install: async () => "sudo apt install -y glslc",
  },
  {
    name: "fuse2",
    desc: "FUSE 2 library (required for AppImage bundling)",
    check: async () => await checkApt("libfuse2"),
    install: async () => "sudo apt install -y libfuse2",
  },
  {
    name: "patchelf",
    desc: "PatchELF utility (required for AppImage bundling)",
    check: async () => await isCommandInPath("patchelf"),
    install: async () => "sudo apt install -y patchelf",
  },
  {
    name: "file",
    desc: "File utility (required for AppImage bundling)",
    check: async () => await isCommandInPath("file"),
    install: async () => "sudo apt install -y file",
  },
  {
    name: "squashfs-tools",
    desc: "SquashFS utilities (required for AppImage bundling)",
    check: async () => await isCommandInPath("mksquashfs"),
    install: async () => "sudo apt install -y squashfs-tools",
  },
  {
    name: "alsa",
    desc: "ALSA audio development headers",
    check: async () => await checkApt("libasound2-dev"),
    install: async () => "sudo apt install -y libasound2-dev",
  },
  {
    name: "openssl",
    desc: "OpenSSL development headers",
    check: async () => await checkApt("libssl-dev"),
    install: async () => "sudo apt install -y libssl-dev",
  },
  {
    name: "libudev",
    desc: "libudev development headers",
    check: async () => await checkApt("libudev-dev"),
    install: async () => "sudo apt install -y libudev-dev",
  },
  {
    name: "rust",
    desc: "Rust toolchain (cargo, rustc)",
    check: async () => await isCommandInPath("cargo"),
    install: async () => "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y",
  },
  {
    name: "nodejs",
    desc: "Node.js runtime (required for UI build)",
    check: async () => await isCommandInPath("node"),
    install: async () => "sudo apt install -y nodejs npm",
  },
];
