// Shared helpers for the project hooks. Built-in Node modules only.
// Every hook must exit 0 on bad input, missing tools or its own bugs:
// a broken hook may never block work.

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const WINDOWS_PATH = /^(?:[a-zA-Z]:[\\/]|\\\\)/;

/**
 * Runs `main(input)` with parsed stdin JSON and always exits 0.
 * `watchdogMs` must stay below the hook `timeout` in .claude/settings.json.
 */
export function runHook(main, { watchdogMs = 8000 } = {}) {
  const exit = () => process.exit(0);
  setTimeout(exit, watchdogMs).unref();
  process.on('uncaughtException', exit);
  process.on('unhandledRejection', exit);
  readInput(Math.min(3000, watchdogMs))
    .then((input) => (input ? main(input) : undefined))
    .then(exit, exit);
}

function readInput(timeoutMs) {
  return new Promise((resolve) => {
    const chunks = [];
    let done = false;
    const finish = () => {
      if (done) return;
      done = true;
      clearTimeout(timer);
      try {
        const value = JSON.parse(Buffer.concat(chunks).toString('utf8'));
        resolve(value && typeof value === 'object' ? value : null);
      } catch {
        resolve(null);
      }
    };
    const timer = setTimeout(finish, timeoutMs);
    process.stdin.on('data', (chunk) => chunks.push(chunk));
    process.stdin.on('end', finish);
    process.stdin.on('error', finish);
  });
}

function writeStdout(text) {
  try {
    fs.writeSync(1, text);
  } catch {
    process.stdout.write(text);
  }
}

/** PreToolUse decision: 'deny' or 'ask', with a reason shown to the user and Claude. */
export function decide(decision, reason) {
  writeStdout(
    JSON.stringify({
      hookSpecificOutput: {
        hookEventName: 'PreToolUse',
        permissionDecision: decision,
        permissionDecisionReason: reason,
      },
    }),
  );
}

/** Windows-style paths use path.win32 even when tests run on Linux. */
export function pathApi(...samples) {
  const windows =
    process.platform === 'win32' || samples.some((p) => typeof p === 'string' && WINDOWS_PATH.test(p));
  return windows ? path.win32 : path.posix;
}

export function projectRoot(input) {
  return process.env.CLAUDE_PROJECT_DIR || input.cwd || process.cwd();
}

/**
 * Path of `target` relative to `root` with `/` separators, '' for the root
 * itself, or null when `target` lies outside `root` (or on another drive).
 */
export function relativeInside(root, target, base = root) {
  const api = pathApi(root, target, base);
  const rel = api.relative(api.resolve(root), api.resolve(base, target));
  if (rel === '') return '';
  if (rel === '..' || rel.startsWith(`..${api.sep}`) || api.isAbsolute(rel)) return null;
  return rel.split(api.sep).join('/');
}

/** Case-insensitive, so `Docs/Input` on Windows counts as `docs/input`. */
export function isUnder(relPath, dir) {
  const rel = relPath.toLowerCase();
  return rel === dir || rel.startsWith(`${dir}/`);
}

const TEST_DIR = /^(?:tests?|__tests__|__snapshots__)$/i;
const DOTNET_TEST_PROJECT_DIR = /\.(?:unit|integration|functional|e2e)?tests?$/i;
const TEST_FILE = [
  /\.(?:test|spec)\.[cm]?[jt]sx?$/i, // TypeScript / JavaScript
  /^tests\.rs$|_tests?\.rs$/i, // Rust test modules in separate files
  /Tests?\.cs$/, // C#: FooTests.cs, FooTest.cs
  /\.snap$/i, // snapshot files
  /\.verified\.[^/]+$/i, // approved snapshot files
];

/** Same patterns as `paths` in .claude/rules/tests.md. Keep both in sync. */
export function isTestFile(relPath) {
  const parts = relPath.split('/');
  const base = parts.pop();
  if (parts.some((dir) => TEST_DIR.test(dir) || DOTNET_TEST_PROJECT_DIR.test(dir))) return true;
  return TEST_FILE.some((pattern) => pattern.test(base));
}

/**
 * Absolute path of an executable on PATH, or null. On Windows only `.exe`
 * counts: `.cmd`/`.bat` shims can't be spawned without a shell.
 */
export function which(command) {
  const name = process.platform === 'win32' ? `${command}.exe` : command;
  for (const dir of (process.env.PATH || '').split(path.delimiter)) {
    if (!dir) continue;
    const candidate = path.join(dir, name);
    try {
      if (fs.statSync(candidate).isFile()) return candidate;
    } catch {
      // not in this directory
    }
  }
  return null;
}

/** Nearest directory from `startDir` up to `root` (inclusive) where `predicate` holds. */
export function findUp(startDir, root, predicate) {
  const top = path.resolve(root);
  let dir = path.resolve(startDir);
  for (;;) {
    if (predicate(dir)) return dir;
    if (dir === top) return null;
    const parent = path.dirname(dir);
    if (parent === dir || relativeInside(top, parent) === null) return null;
    dir = parent;
  }
}

export function exists(filePath) {
  try {
    fs.accessSync(filePath);
    return true;
  } catch {
    return false;
  }
}

export const homeDir = os.homedir();
