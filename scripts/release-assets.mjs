// Single source of truth for release identity: the version, the git tag, the
// release asset file names, and their download URLs. package-release.mjs names
// the build outputs from here and release-notes-template.mjs writes the
// Downloads block from here, so the two can never disagree.
//
// install.sh, install.ps1 and the in-app updater derive download URLs from the
// tag and these exact file names, so a wrong name breaks updates and installs.

import { readFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const TAURI_CONF = JSON.parse(readFileSync(resolve(ROOT, "src-tauri", "tauri.conf.json"), "utf8"));

const REPOSITORY = "jackbrumley/voquill";

export const VERSION = TAURI_CONF.version;
export const TAG = `v${VERSION}`;

export const ASSET_NAMES = {
  windowsSetup: `voquill-${VERSION}-windows-x64-setup.exe`,
  windowsMsi: `voquill-${VERSION}-windows-x64.msi`,
  linuxDeb: `voquill-${VERSION}-linux-x64.deb`,
  linuxRpm: `voquill-${VERSION}-linux-x64.rpm`,
  linuxAppImage: `voquill-${VERSION}-linux-x64.AppImage`,
};

// Order and labels of the Downloads block in every release's notes.
export const DOWNLOAD_LINKS = [
  { platform: "Windows", label: "Setup EXE (Recommended)", fileName: ASSET_NAMES.windowsSetup },
  { platform: "Windows", label: "System MSI (IT / Admin)", fileName: ASSET_NAMES.windowsMsi },
  { platform: "Linux (Debian/Ubuntu)", label: ".deb", fileName: ASSET_NAMES.linuxDeb },
  { platform: "Linux (Fedora/RHEL)", label: ".rpm", fileName: ASSET_NAMES.linuxRpm },
  { platform: "Linux (Portable)", label: ".AppImage", fileName: ASSET_NAMES.linuxAppImage },
];

// The tag (with its `v`) is the URL path segment; the file name carries the
// bare version.
export function downloadUrl(fileName) {
  return `https://github.com/${REPOSITORY}/releases/download/${TAG}/${fileName}`;
}

// Asset names are `voquill-<version>-<os>-<arch>[-setup].<ext>`. A plain
// MAJOR.MINOR.PATCH version keeps them unambiguous (a suffix like `-beta.1`
// would add hyphens that collide with the separators).
export function assertPlainVersion() {
  if (!/^\d+\.\d+\.\d+$/.test(VERSION)) {
    console.error(`Release version '${VERSION}' must be plain MAJOR.MINOR.PATCH (no pre-release or build suffix).`);
    process.exit(1);
  }
}
