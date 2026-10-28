#!/usr/bin/env node

// Step one of a release: sets the version in every file that carries it,
// refreshes both lockfiles, commits exactly those files and creates the tag.
// Pushing the tag (printed at the end) starts the release workflow.
//
//   npm run release:prepare -- 1.8.0
//
// Must run on a `dev` checkout. Other uncommitted work is left untouched.

import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const RELEASE_BRANCH = "dev";

// Each file holding the version, with the pattern that matches its single
// top-level version field.
const VERSION_FILES = [
  { path: "package.json", pattern: /^(  "version": ")[^"]+(",)$/m },
  { path: "src-tauri/tauri.conf.json", pattern: /^(  "version": ")[^"]+(",)$/m },
  { path: "src-tauri/Cargo.toml", pattern: /^(version = ")[^"]+(")$/m },
];
const LOCKFILES = ["package-lock.json", "src-tauri/Cargo.lock"];

function fail(message) {
  console.error(`\nRelease preparation failed: ${message}`);
  process.exit(1);
}

function run(command, args) {
  console.log(`$ ${command} ${args.join(" ")}`);
  execFileSync(command, args, { cwd: ROOT, stdio: "inherit" });
}

function git(...args) {
  return execFileSync("git", args, { cwd: ROOT, encoding: "utf8" }).trim();
}

function setVersion({ path, pattern }, version) {
  const filePath = resolve(ROOT, path);
  const contents = readFileSync(filePath, "utf8");
  if (!pattern.test(contents)) {
    fail(`could not find the version field in ${path}`);
  }
  writeFileSync(filePath, contents.replace(pattern, `$1${version}$2`));
}

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) {
  fail("usage: npm run release:prepare -- X.Y.Z (plain MAJOR.MINOR.PATCH, no suffix)");
}
const tag = `v${version}`;

const branch = git("rev-parse", "--abbrev-ref", "HEAD");
if (branch !== RELEASE_BRANCH) {
  fail(`releases are tagged on '${RELEASE_BRANCH}', but '${branch}' is checked out`);
}
if (git("tag", "--list", tag)) {
  fail(`tag ${tag} already exists`);
}
const releaseFiles = [...VERSION_FILES.map((file) => file.path), ...LOCKFILES];
if (git("status", "--porcelain", "--", ...releaseFiles)) {
  fail(`commit or discard existing changes to ${releaseFiles.join(", ")} first`);
}

for (const file of VERSION_FILES) {
  setVersion(file, version);
}
run("npm", ["install", "--package-lock-only"]);
run("cargo", ["update", "--workspace", "--manifest-path", "src-tauri/Cargo.toml"]);

run("git", ["add", ...releaseFiles]);
run("git", ["commit", "-m", `chore: bump version to ${version}`]);
run("git", ["tag", tag]);

console.log(`\nPrepared ${tag}. Start the release workflow with:\n\n  git push origin ${RELEASE_BRANCH} ${tag}\n`);
