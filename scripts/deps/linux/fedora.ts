import { Dependency } from "../types.ts";
import { isCommandInPath, checkPkgConfig, checkRpm } from "../utils.ts";

export const getFedoraDependencies = (): Dependency[] => [
  {
    name: "libpulse",
    desc: "PulseAudio development headers",
    check: async () => await checkPkgConfig("libpulse"),
    install: async () => "sudo dnf install -y pulseaudio-libs-devel",
  },
  {
    name: "libgtk-layer-shell",
    desc: "GTK Layer Shell development headers",
    check: async () => await checkPkgConfig("gtk-layer-shell-0"),
    install: async () => "sudo dnf install -y gtk-layer-shell-devel",
  },
  {
    name: "cmake",
    desc: "CMake build system",
    check: async () => await isCommandInPath("cmake"),
    install: async () => "sudo dnf install -y cmake",
  },
  {
    name: "pkg-config",
    desc: "Package configuration tool",
    check: async () => await isCommandInPath("pkg-config"),
    install: async () => "sudo dnf install -y pkgconf-pkg-config",
  },
  {
    name: "libclang",
    desc: "Clang development headers",
    check: async () => await checkRpm("clang-devel"),
    install: async () => "sudo dnf install -y clang-devel",
  },
  {
    name: "build-essential",
    desc: "Build tools (gcc, g++, make, etc.)",
    check: async () => await checkRpm("gcc-c++") && await isCommandInPath("make"),
    install: async () => "sudo dnf install -y gcc-c++ make",
  },
  {
    name: "glibc-headers",
    desc: "C Standard Library development headers",
    check: async () => await checkRpm("glibc-devel"),
    install: async () => "sudo dnf install -y glibc-devel",
  },
  {
    name: "wl-clipboard",
    desc: "Wayland clipboard utilities",
    check: async () => await isCommandInPath("wl-copy"),
    install: async () => "sudo dnf install -y wl-clipboard",
  },
  {
    name: "libwebkit2gtk-4.1",
    desc: "WebKitGTK development headers",
    check: async () => await checkPkgConfig("webkit2gtk-4.1"),
    install: async () => "sudo dnf install -y webkit2gtk4.1-devel",
  },
  {
    name: "libgtk-3",
    desc: "GTK3 development headers",
    check: async () => await checkPkgConfig("gtk+-3.0"),
    install: async () => "sudo dnf install -y gtk3-devel",
  },
  {
    name: "libayatana-appindicator3",
    desc: "Ayatana AppIndicator (required for Tauri tray icons)",
    check: async () => await checkPkgConfig("ayatana-appindicator3-0.1"),
    install: async () => "sudo dnf install -y libayatana-appindicator-gtk3-devel",
  },
  {
    name: "librsvg2",
    desc: "librsvg development headers",
    check: async () => await checkPkgConfig("librsvg-2.0"),
    install: async () => "sudo dnf install -y librsvg2-devel",
  },
  {
    name: "vulkan-headers",
    desc: "Vulkan development headers (required for Turbo Mode)",
    check: async () => await checkRpm("vulkan-headers"),
    install: async () => "sudo dnf install -y vulkan-headers vulkan-loader-devel",
  },
  {
    name: "shaderc",
    desc: "Vulkan shader compiler (required for Turbo Mode)",
    check: async () => await isCommandInPath("glslc"),
    install: async () => "sudo dnf install -y glslc",
  },
  {
    name: "fuse2",
    desc: "FUSE 2 library (required for AppImage bundling)",
    check: async () => await checkRpm("fuse-libs"),
    install: async () => "sudo dnf install -y fuse-libs",
  },
  {
    name: "patchelf",
    desc: "PatchELF utility (required for AppImage bundling)",
    check: async () => await isCommandInPath("patchelf"),
    install: async () => "sudo dnf install -y patchelf",
  },
  {
    name: "file",
    desc: "File utility (required for AppImage bundling)",
    check: async () => await isCommandInPath("file"),
    install: async () => "sudo dnf install -y file",
  },
  {
    name: "squashfs-tools",
    desc: "SquashFS utilities (required for AppImage bundling)",
    check: async () => await isCommandInPath("mksquashfs"),
    install: async () => "sudo dnf install -y squashfs-tools",
  },
  {
    name: "alsa",
    desc: "ALSA audio development headers",
    check: async () => await checkPkgConfig("alsa"),
    install: async () => "sudo dnf install -y alsa-lib-devel",
  },
  {
    name: "openssl",
    desc: "OpenSSL development headers",
    check: async () => await checkPkgConfig("openssl"),
    install: async () => "sudo dnf install -y openssl-devel",
  },
  {
    name: "libudev",
    desc: "libudev development headers",
    check: async () => await checkPkgConfig("libudev"),
    install: async () => "sudo dnf install -y systemd-devel",
  },
  {
    name: "rust",
    desc: "Rust toolchain (cargo, rustc)",
    check: async () => await isCommandInPath("cargo"),
    install: async () => "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && source $HOME/.cargo/env",
  },
  {
    name: "libxcrypt-compat",
    desc: "Backward compatibility for libcrypt.so.1",
    check: async () => await checkRpm("libxcrypt-compat"),
    install: async () => "sudo dnf install -y libxcrypt-compat",
  },
  {
    name: "nodejs",
    desc: "Node.js runtime (required for UI build)",
    check: async () => await isCommandInPath("node"),
    install: async () => "sudo dnf install -y nodejs npm",
  },
];
