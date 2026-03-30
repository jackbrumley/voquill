import { Dependency } from "../types.ts";
import { isCommandInPath } from "../utils.ts";
import { exists } from "jsr:@std/fs";

async function checkDefaultPath(paths: string[]): Promise<boolean> {
  for (const path of paths) {
    if (await exists(path)) return true;
  }
  return false;
}

export const getWindowsDependencies = (): Dependency[] => [
  {
    name: "clang",
    desc: "LLVM/Clang (required for bindgen)",
    check: async () => await isCommandInPath("clang") || await checkDefaultPath([
      "C:\\Program Files\\LLVM\\bin\\clang.exe",
      "C:\\Program Files (x86)\\LLVM\\bin\\clang.exe"
    ]),
    install: async () => "winget install -e --id LLVM.LLVM",
  },
  {
    name: "cmake",
    desc: "CMake (required for building C/C++ libs)",
    check: async () => await isCommandInPath("cmake") || await checkDefaultPath([
      "C:\\Program Files\\CMake\\bin\\cmake.exe",
      "C:\\Program Files (x86)\\CMake\\bin\\cmake.exe"
    ]),
    install: async () => "winget install -e --id Kitware.CMake",
  },
  {
    name: "rust",
    desc: "Rust toolchain (cargo, rustc)",
    check: async () => await isCommandInPath("cargo"),
    install: async () => "winget install -e --id Rustlang.Rustup",
  },
  {
    name: "nodejs",
    desc: "Node.js runtime (required for UI build)",
    check: async () => await isCommandInPath("node"),
    install: async () => "winget install -e --id OpenJS.NodeJS",
  },
];
