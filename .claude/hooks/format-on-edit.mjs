// PostToolUse (Edit|Write|MultiEdit): formats the edited file, but only when both
// the formatter and the project configuration exist. Otherwise it does nothing.
//   .rs  -> rustfmt via stdin (formats only this file), needs a Cargo.toml above it
//   TS/JS/CSS/JSON -> local node_modules prettier, needs a prettier config above it
//   .cs  -> `dotnet format whitespace`, needs a .csproj above it
// Formatter failures are ignored: syntax errors surface via LSP diagnostics and builds.

import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { exists, findUp, projectRoot, relativeInside, runHook, which } from './_lib.mjs';

const PRETTIER_EXTENSIONS = new Set([
  '.ts', '.tsx', '.mts', '.cts', '.js', '.jsx', '.mjs', '.cjs', '.json', '.css', '.scss',
]);
const PRETTIER_CONFIGS = [
  '.prettierrc', '.prettierrc.json', '.prettierrc.json5', '.prettierrc.yaml', '.prettierrc.yml',
  '.prettierrc.toml', '.prettierrc.js', '.prettierrc.cjs', '.prettierrc.mjs', '.prettierrc.ts',
  '.prettierrc.mts', '.prettierrc.cts', 'prettier.config.js', 'prettier.config.cjs',
  'prettier.config.mjs', 'prettier.config.ts', 'prettier.config.mts', 'prettier.config.cts',
];
const SPAWN_TIMEOUT_MS = 45_000;

runHook(
  (input) => {
    const filePath = input.tool_input?.file_path;
    if (typeof filePath !== 'string' || filePath === '') return;

    const root = path.resolve(projectRoot(input));
    if (relativeInside(root, filePath) === null) return;
    const file = path.resolve(root, filePath);
    if (!exists(file)) return;

    const extension = path.extname(file).toLowerCase();
    if (extension === '.rs') formatRust(file, root);
    else if (PRETTIER_EXTENSIONS.has(extension)) formatWithPrettier(file, root);
    else if (extension === '.cs') formatCSharp(file, root);
  },
  { watchdogMs: 55_000 },
);

function formatRust(file, root) {
  const dir = path.dirname(file);
  if (!findUp(dir, root, (d) => exists(path.join(d, 'Cargo.toml')))) return;
  const rustfmt = which('rustfmt');
  if (!rustfmt) return;

  const source = fs.readFileSync(file, 'utf8');
  const edition = readCargoEdition(dir, root);
  const args = ['--emit', 'stdout', ...(edition ? ['--edition', edition] : [])];
  // stdin mode formats this file only; a path argument would also reformat its `mod` children.
  const result = spawnSync(rustfmt, args, {
    cwd: dir,
    input: source,
    encoding: 'utf8',
    timeout: SPAWN_TIMEOUT_MS,
    windowsHide: true,
  });
  if (result.status === 0 && result.stdout && result.stdout !== source) {
    fs.writeFileSync(file, result.stdout);
  }
}

function readCargoEdition(dir, root) {
  let edition = null;
  findUp(dir, root, (d) => {
    try {
      const match = /^\s*edition\s*=\s*"(\d{4})"/m.exec(fs.readFileSync(path.join(d, 'Cargo.toml'), 'utf8'));
      if (match) edition = match[1];
      return Boolean(match);
    } catch {
      return false;
    }
  });
  return edition;
}

function formatWithPrettier(file, root) {
  const dir = path.dirname(file);
  if (!findUp(dir, root, hasPrettierConfig)) return;
  const prettier = findLocalPrettier(dir, root);
  if (!prettier) return;
  // Run prettier's JS entry with this Node: no npx downloads, no .cmd shims on Windows.
  spawnSync(process.execPath, [prettier, '--write', file], {
    cwd: dir,
    stdio: 'ignore',
    timeout: SPAWN_TIMEOUT_MS,
    windowsHide: true,
  });
}

function hasPrettierConfig(dir) {
  if (PRETTIER_CONFIGS.some((name) => exists(path.join(dir, name)))) return true;
  try {
    return 'prettier' in JSON.parse(fs.readFileSync(path.join(dir, 'package.json'), 'utf8'));
  } catch {
    return false;
  }
}

function findLocalPrettier(dir, root) {
  const home = findUp(dir, root, (d) => exists(path.join(d, 'node_modules', 'prettier', 'package.json')));
  if (!home) return null;
  const packageDir = path.join(home, 'node_modules', 'prettier');
  try {
    const { bin } = JSON.parse(fs.readFileSync(path.join(packageDir, 'package.json'), 'utf8'));
    const entry = typeof bin === 'string' ? bin : bin?.prettier;
    return entry ? path.join(packageDir, entry) : null;
  } catch {
    return null;
  }
}

function formatCSharp(file, root) {
  const projectDir = findUp(path.dirname(file), root, (d) => {
    try {
      return fs.readdirSync(d).some((name) => name.toLowerCase().endsWith('.csproj'));
    } catch {
      return false;
    }
  });
  if (!projectDir) return;
  const dotnet = which('dotnet');
  if (!dotnet) return;
  // `whitespace --folder` needs no restore or build; style comes from .editorconfig.
  spawnSync(
    dotnet,
    ['format', 'whitespace', '.', '--folder', '--include', path.relative(projectDir, file)],
    { cwd: projectDir, stdio: 'ignore', timeout: SPAWN_TIMEOUT_MS, windowsHide: true },
  );
}
