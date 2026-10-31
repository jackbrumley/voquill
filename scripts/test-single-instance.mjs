#!/usr/bin/env node

// Checks the single-instance guard on an installed build: a second launch must
// hand off to the running instance and exit, and the running instance must
// survive it (on Linux, a second launch used to crash the first). The release
// workflow runs this after installing on each platform.
//
//   node scripts/test-single-instance.mjs <path to the installed voquill binary>

import { spawn } from "node:child_process";
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

// Startup ends with setup, which logs tray availability. A fresh CI runner can
// take 20s+ before that (Windows creates the WebView2 window first).
const STARTUP_TIMEOUT_MS = 120_000;
const STARTUP_COMPLETE_LINE = "System tray available";
const SECOND_LAUNCH_TIMEOUT_MS = 30_000;
const SURVIVAL_WAIT_MS = 5_000;
const SESSION_LOG = join(homedir(), ".config", "voquill-app", "debug", "session.log");

const binary = process.argv[2];
if (!binary) {
  console.error("Usage: node scripts/test-single-instance.mjs <voquill binary>");
  process.exit(2);
}

const sleep = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));

function launch() {
  const child = spawn(binary, [], { stdio: "ignore" });
  child.exited = new Promise((resolve) => child.on("exit", (code, signal) => resolve({ code, signal })));
  return child;
}

function readSessionLog() {
  try {
    return readFileSync(SESSION_LOG, "utf8");
  } catch {
    return "";
  }
}

async function waitForStartup(exitedEarly) {
  const deadline = Date.now() + STARTUP_TIMEOUT_MS;
  while (Date.now() < deadline && !exitedEarly()) {
    if (readSessionLog().includes(STARTUP_COMPLETE_LINE)) {
      return true;
    }
    await sleep(500);
  }
  return false;
}

function fail(message) {
  console.error(`FAIL: ${message}`);
  console.error(`--- ${SESSION_LOG} ---\n${readSessionLog()}`);
  process.exitCode = 1;
}

const first = launch();
let firstExit = null;
first.exited.then((result) => (firstExit = result));
const started = await waitForStartup(() => firstExit !== null);

if (firstExit) {
  fail(`first instance exited during startup (${JSON.stringify(firstExit)})`);
} else if (!started) {
  fail(`first instance did not finish starting within ${STARTUP_TIMEOUT_MS / 1000}s`);
} else {
  const second = launch();
  const secondExit = await Promise.race([second.exited, sleep(SECOND_LAUNCH_TIMEOUT_MS).then(() => null)]);
  await sleep(SURVIVAL_WAIT_MS);
  const sessionLog = readSessionLog();

  if (!secondExit) {
    second.kill();
    fail("second launch kept running instead of handing off to the first instance");
  } else if (secondExit.code !== 0) {
    fail(`second launch exited with ${JSON.stringify(secondExit)}`);
  } else if (firstExit) {
    fail(`first instance exited after the second launch (${JSON.stringify(firstExit)})`);
  } else if (!sessionLog.includes("Second launch detected")) {
    fail("first instance did not log the second launch hand-off");
  } else if (sessionLog.includes("PANIC")) {
    fail("session log records a panic");
  } else {
    console.log("PASS: second launch handed off to the running instance, which kept running");
  }
}

first.kill();
await first.exited;
