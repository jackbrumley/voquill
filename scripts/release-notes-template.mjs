#!/usr/bin/env node

// Prints the release notes skeleton for the version in tauri.conf.json to
// stdout, with the Downloads block generated from release-assets.mjs so links
// can never be mistyped. The commits since the previous tag are printed to
// stderr as source material for the What's New sections.
//
//   npm run --silent release:notes > release-notes.md
//
// Every `TODO` line must be replaced before the notes are published.

import { execFileSync } from "node:child_process";
import { DOWNLOAD_LINKS, TAG, assertPlainVersion, downloadUrl } from "./release-assets.mjs";

function git(...args) {
  return execFileSync("git", args, { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }).trim();
}

function tagExists(tag) {
  try {
    git("rev-parse", "--verify", "--quiet", `refs/tags/${tag}`);
    return true;
  } catch {
    return false;
  }
}

// The release range ends at this release's tag once it exists, otherwise at
// HEAD; it starts at the nearest earlier tag.
function releaseRange() {
  const end = tagExists(TAG) ? TAG : "HEAD";
  const previousTag = git("describe", "--tags", "--abbrev=0", `${end}^`);
  return { previousTag, end };
}

function printCommitsSincePreviousRelease() {
  const { previousTag, end } = releaseRange();
  const subjects = git("log", "--no-merges", "--format=- %s", `${previousTag}..${end}`);
  console.error(`Commits in ${previousTag}..${end} (source material, not part of the notes):\n`);
  console.error(`${subjects}\n`);
}

function notesTemplate() {
  const downloads = DOWNLOAD_LINKS
    .map((link) => `- ${link.platform}: [${link.label}](${downloadUrl(link.fileName)})`)
    .join("\n");

  return `# Voquill ${TAG}

TODO: One paragraph: "Voquill ${TAG} is a ... release, ..." summarising the headline changes.

---

## What's New

### TODO: Theme Name
- TODO Label: One user-facing sentence about the change.
- TODO Label: Another change in this theme.

---

### TODO: Theme Name
- TODO Label: One user-facing sentence about the change.

---

## Downloads

${downloads}
`;
}

assertPlainVersion();
printCommitsSincePreviousRelease();
process.stdout.write(notesTemplate());
