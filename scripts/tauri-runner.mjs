#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { logError, run } from "@tauri-apps/cli/main.js";

if (process.platform === "win32") {
  process.env.CARGO_TARGET_DIR = "C:\\vb";
  // Build whisper.cpp with Ninja instead of the default MSBuild generator.
  // MSBuild's tracked CustomBuild steps can break nested CMake try_compile
  // checks (ggml-vulkan builds vulkan-shaders-gen via ExternalProject), and
  // Ninja builds faster.
  process.env.CMAKE_GENERATOR = "Ninja";
}

process.env.GGML_NATIVE = "OFF";
process.env.GGML_AVX512 = "OFF";
process.env.GGML_AVX512_VBMI = "OFF";
process.env.GGML_AVX512_VNNI = "OFF";
process.env.GGML_AVX512_BF16 = "OFF";
process.env.GGML_AMX_TILE = "OFF";
process.env.GGML_AMX_INT8 = "OFF";
process.env.GGML_AMX_BF16 = "OFF";
process.env.GGML_AVX_VNNI = "OFF";

// whisper-rs-sys compiles whisper.cpp/ggml natively but does not declare
// cargo:rerun-if-env-changed for the GGML_* variables above, so changing them
// never invalidates Cargo's build cache. Without a forced rebuild, stale
// objects (e.g. compiled with /arch:AVX512 from a previous GGML_NATIVE=ON
// build) get silently relinked into the release binary. Clean the package
// before every release build so these flags always reach the compiler.
// NOTE: cargo clean -p without --release only cleans dev-profile artifacts,
// so --release is required here to match the `tauri build` profile.
if (process.argv[2] === "build") {
  execFileSync("cargo", ["clean", "-p", "whisper-rs-sys", "--release"], {
    cwd: new URL("../src-tauri", import.meta.url),
    env: process.env,
    stdio: "inherit",
  });
}

// The deb/rpm desktop template renders a Hidden stub (the real launcher ships
// as org.voquill.desktop.desktop), and the AppImage reuses that template. The
// AppImage overrides the stub via bundle.linux.appimage.files, which relies on
// the bundler's generated filename. Fail the build if that ever drifts and the
// AppImage's top-level launcher ends up hidden.
function verifyAppImageLauncher(buildStartedAt) {
  const targetDir = process.env.CARGO_TARGET_DIR
    ?? fileURLToPath(new URL("../src-tauri/target", import.meta.url));
  const appImageBundleDir = join(targetDir, "release", "bundle", "appimage");
  const appDirs = existsSync(appImageBundleDir)
    ? readdirSync(appImageBundleDir)
      .filter((entry) => entry.endsWith(".AppDir"))
      .map((entry) => join(appImageBundleDir, entry))
      .filter((appDir) => statSync(appDir).mtimeMs >= buildStartedAt)
    : [];

  if (appDirs.length === 0) {
    console.log("AppImage launcher check skipped: no AppImage was bundled in this build.");
    return;
  }

  for (const appDir of appDirs) {
    const launchers = readdirSync(appDir).filter((entry) => entry.endsWith(".desktop"));
    if (launchers.length === 0) {
      throw new Error(`AppImage launcher check failed: no top-level .desktop entry in ${appDir}`);
    }
    for (const launcher of launchers) {
      const contents = readFileSync(join(appDir, launcher), "utf8");
      if (/^Hidden=true$/m.test(contents)) {
        throw new Error(
          `AppImage launcher check failed: ${join(appDir, launcher)} is the hidden bundler stub. `
          + "Update bundle.linux.appimage.files in tauri.conf.json to the bundler's generated desktop filename.",
        );
      }
    }
    console.log(`AppImage launcher check passed: ${launchers.join(", ")} in ${appDir}`);
  }
}

try {
  const buildStartedAt = Date.now();
  await run(process.argv.slice(2), "tauri");
  if (process.argv[2] === "build" && process.platform === "linux") {
    verifyAppImageLauncher(buildStartedAt);
  }
} catch (error) {
  const message = error instanceof Error ? error.message : String(error);
  if (typeof logError === "function") logError(message);
  console.error(error);
  process.exit(1);
}
