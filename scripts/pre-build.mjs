#!/usr/bin/env bun
/**
 * pre-build — Tauri beforeBuildCommand wrapper (cross-platform, no shell `&&`).
 *
 * Runs prune-build --pre first, then builds the UI. This is a single script so
 * it avoids shell-chaining issues on Windows (cmd.exe `&&` edge cases).
 */
import { execSync } from 'node:child_process';

function run(cmd) {
  console.log(`[pre-build] ${cmd}`);
  execSync(cmd, { stdio: 'inherit', cwd: new URL('..', import.meta.url).pathname });
}

run('bun scripts/prune-build.mjs --pre');
run('bun run --filter=./ui build');