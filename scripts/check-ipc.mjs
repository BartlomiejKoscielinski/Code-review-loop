// Fails when the TypeScript IPC types in packages/ipc are out of date.
// `cargo test` regenerates them (ts-rs), so a difference afterwards means a
// Rust IPC type changed without the regenerated types being added.
import { spawnSync } from 'node:child_process';
import { rmSync } from 'node:fs';

const GENERATED = 'packages/ipc/src/generated';

function run(command, args) {
  const result = spawnSync(command, args, {
    maxBuffer: 64 * 1024 * 1024,
    stdio: ['ignore', 'pipe', 'inherit'],
    encoding: 'utf8',
  });
  if (result.error) throw result.error;
  return result;
}

// ts-rs never deletes files, so start empty: a removed Rust type then shows
// up as a deleted file.
rmSync(GENERATED, { recursive: true, force: true });
const test = run('cargo', ['test', '--workspace', '--lib', '--quiet']);
if (test.status !== 0) {
  process.stdout.write(test.stdout);
  console.error('check:ipc: cargo test failed');
  process.exit(1);
}

// Compare the working tree with the index (staged files count as committed
// here) and catch new files that were never added.
const changed = run('git', ['diff', '--name-only', '--', GENERATED]);
const untracked = run('git', ['ls-files', '--others', '--exclude-standard', '--', GENERATED]);
if (changed.status !== 0 || untracked.status !== 0) {
  console.error('check:ipc: git failed');
  process.exit(1);
}
const stale = `${changed.stdout}${untracked.stdout}`.trim();
if (stale !== '') {
  console.error(`check:ipc: IPC types changed. Commit these files:\n${stale}`);
  process.exit(1);
}
console.log('check:ipc: IPC types are up to date');
