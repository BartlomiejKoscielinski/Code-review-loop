// Tests for the project hooks: node --test .claude/hooks/hooks.test.mjs
// Each hook runs as a child process with JSON on stdin, as Claude Code runs it.

import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { after, before, describe, test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { which } from './_lib.mjs';

const HOOKS_DIR = path.dirname(fileURLToPath(import.meta.url));
const WINDOWS_PROJECT = 'C:\\Work\\lupa';
let project;

const RUST_LIB = [
  'pub fn add(a: i32, b: i32) -> i32 {',
  '    a + b',
  '}',
  '',
  '#[cfg(test)]',
  'mod tests {',
  '    use super::*;',
  '',
  '    #[test]',
  '    fn adds() {',
  '        assert_eq!(add(2, 2), 4);',
  '    }',
  '}',
  '',
].join('\n');

function writeFixture(relPath, content = '') {
  const file = path.join(project, relPath);
  fs.mkdirSync(path.dirname(file), { recursive: true });
  fs.writeFileSync(file, content);
  return file;
}

function runHook(name, stdin, { projectDir = project, env = {} } = {}) {
  const result = spawnSync(process.execPath, [path.join(HOOKS_DIR, name)], {
    input: typeof stdin === 'string' ? stdin : JSON.stringify(stdin),
    encoding: 'utf8',
    env: { ...process.env, CLAUDE_PROJECT_DIR: projectDir, ...env },
    timeout: 60_000,
  });
  assert.equal(result.status, 0, `exit code (stderr: ${result.stderr})`);
  return result.stdout.trim() === '' ? null : JSON.parse(result.stdout).hookSpecificOutput;
}

const preToolUse = (toolName, toolInput, cwd = project) => ({
  hook_event_name: 'PreToolUse',
  tool_name: toolName,
  tool_input: toolInput,
  cwd,
});

before(() => {
  project = fs.realpathSync(fs.mkdtempSync(path.join(os.tmpdir(), 'lupa-hooks-')));
  writeFixture('docs/input/plan.md', '# plan\n');
  writeFixture('ui/src/app.test.ts', "test('x', () => {});\n");
  writeFixture('ui/src/app.ts', 'export const x = 1;\n');
  writeFixture('core/src/lib.rs', RUST_LIB);
  writeFixture('core/tests/integration.rs', '#[test]\nfn works() {}\n');
  writeFixture('sidecar/Lupa.Sidecar.Tests/ParserTests.cs', 'public class ParserTests {}\n');
  writeFixture('notes.txt', 'plain text\n');
});

after(() => fs.rmSync(project, { recursive: true, force: true }));

describe('protect-inputs', () => {
  test('denies writing into docs/input/', () => {
    const out = runHook('protect-inputs.mjs', preToolUse('Write', { file_path: path.join(project, 'docs/input/plan.md'), content: 'x' }));
    assert.equal(out.permissionDecision, 'deny');
    assert.match(out.permissionDecisionReason, /docs\/input/);
  });

  test('denies Windows paths, case-insensitively', () => {
    for (const file of ['C:\\Work\\lupa\\docs\\input\\mocks\\a.png', 'c:\\work\\LUPA\\Docs\\Input\\plan.md']) {
      const out = runHook('protect-inputs.mjs', preToolUse('Edit', { file_path: file, old_string: 'a', new_string: 'b' }, WINDOWS_PROJECT), {
        projectDir: WINDOWS_PROJECT,
      });
      assert.equal(out?.permissionDecision, 'deny', file);
    }
  });

  test('denies NotebookEdit in docs/input/', () => {
    const out = runHook('protect-inputs.mjs', preToolUse('NotebookEdit', { notebook_path: path.join(project, 'docs/input/n.ipynb') }));
    assert.equal(out.permissionDecision, 'deny');
  });

  test('lets other paths through', () => {
    for (const rel of ['docs/plan/roadmap.md', 'docs/inputs.md', 'ui/docs/input/x.md']) {
      assert.equal(runHook('protect-inputs.mjs', preToolUse('Write', { file_path: path.join(project, rel), content: 'x' })), null, rel);
    }
  });
});

describe('guard-tests', () => {
  test('asks before editing an existing test file', () => {
    for (const rel of ['ui/src/app.test.ts', 'core/tests/integration.rs', 'sidecar/Lupa.Sidecar.Tests/ParserTests.cs']) {
      const out = runHook('guard-tests.mjs', preToolUse('Edit', { file_path: path.join(project, rel), old_string: 'x', new_string: 'y' }));
      assert.equal(out?.permissionDecision, 'ask', rel);
    }
  });

  test('lets a new test file through', () => {
    const out = runHook('guard-tests.mjs', preToolUse('Write', { file_path: path.join(project, 'ui/src/new.test.ts'), content: 'test()' }));
    assert.equal(out, null);
  });

  test('lets non-test files through', () => {
    const out = runHook('guard-tests.mjs', preToolUse('Edit', { file_path: path.join(project, 'ui/src/app.ts'), old_string: '1', new_string: '2' }));
    assert.equal(out, null);
  });

  test('Rust: edit before #[cfg(test)] passes, edit inside it asks', () => {
    const file = path.join(project, 'core/src/lib.rs');
    assert.equal(runHook('guard-tests.mjs', preToolUse('Edit', { file_path: file, old_string: '    a + b', new_string: '    b + a' })), null);
    const out = runHook('guard-tests.mjs', preToolUse('Edit', { file_path: file, old_string: 'add(2, 2), 4', new_string: 'add(2, 2), 5' }));
    assert.equal(out?.permissionDecision, 'ask');
  });

  test('Rust: Write keeping the test module passes, changing it asks', () => {
    const file = path.join(project, 'core/src/lib.rs');
    const sameTests = RUST_LIB.replace('a + b', 'b + a');
    assert.equal(runHook('guard-tests.mjs', preToolUse('Write', { file_path: file, content: sameTests })), null);
    const out = runHook('guard-tests.mjs', preToolUse('Write', { file_path: file, content: RUST_LIB.replace('4);', '5);') }));
    assert.equal(out?.permissionDecision, 'ask');
  });
});

describe('format-on-edit', () => {
  const postToolUse = (file) => ({ hook_event_name: 'PostToolUse', tool_name: 'Edit', tool_input: { file_path: file }, cwd: project });

  test('does nothing for a file without a formatter', () => {
    const file = path.join(project, 'notes.txt');
    assert.equal(runHook('format-on-edit.mjs', postToolUse(file)), null);
    assert.equal(fs.readFileSync(file, 'utf8'), 'plain text\n');
  });

  test('does nothing without project config (no prettier config, no Cargo.toml, no .csproj)', () => {
    const files = {
      'loose/a.ts': 'export   const  a=1\n',
      'loose/a.rs': 'fn  main( ){}\n',
      'loose/A.cs': 'class   A{}\n',
    };
    for (const [rel, content] of Object.entries(files)) {
      const file = writeFixture(rel, content);
      runHook('format-on-edit.mjs', postToolUse(file));
      assert.equal(fs.readFileSync(file, 'utf8'), content, rel);
    }
  });

  test('does nothing when the toolchain is missing', () => {
    writeFixture('crate/Cargo.toml', '[package]\nname = "crate"\nedition = "2021"\n');
    const content = 'fn  main( ){}\n';
    const file = writeFixture('crate/src/main.rs', content);
    runHook('format-on-edit.mjs', postToolUse(file), { env: { PATH: '' } });
    assert.equal(fs.readFileSync(file, 'utf8'), content);
  });

  test('formats Rust with rustfmt when Cargo.toml exists', { skip: !which('rustfmt') && 'rustfmt not on PATH' }, () => {
    writeFixture('crate2/Cargo.toml', '[package]\nname = "crate2"\nedition = "2021"\n');
    const file = writeFixture('crate2/src/main.rs', 'async fn  run( ){}\nfn main(){}\n');
    runHook('format-on-edit.mjs', postToolUse(file));
    assert.equal(fs.readFileSync(file, 'utf8'), 'async fn run() {}\nfn main() {}\n');
  });

  test('ignores files outside the project', () => {
    const outside = path.join(os.tmpdir(), 'lupa-outside.rs');
    assert.equal(runHook('format-on-edit.mjs', postToolUse(outside)), null);
  });
});

describe('guard-delete', () => {
  const bash = (command, cwd = project) => runHook('guard-delete.mjs', preToolUse('Bash', { command }, cwd));
  const powershell = (command) =>
    runHook('guard-delete.mjs', preToolUse('PowerShell', { command }, WINDOWS_PROJECT), { projectDir: WINDOWS_PROJECT });

  test('denies deleting outside the repo', () => {
    for (const command of [
      'rm -rf /tmp/lupa-elsewhere',
      'rm -rf ../other',
      'cd .. && rm -rf lupa-other',
      'rm -rf ~',
      'rm -rf /*',
      'find /var/log -name "*.log" -delete',
      "bash -c 'rm -rf /etc/x'",
      'timeout 5 rm -f -- /tmp/a',
    ]) {
      assert.equal(bash(command)?.permissionDecision, 'deny', command);
    }
  });

  test('denies deleting the repo root, .git/ and docs/input/', () => {
    for (const command of ['rm -rf .', 'rm -rf .git', 'rm docs/input/plan.md', `rm -rf "${project}"`]) {
      assert.equal(bash(command)?.permissionDecision, 'deny', command);
    }
  });

  test('allows deleting inside the repo', () => {
    for (const command of [
      'rm -rf target node_modules',
      'rm -f out.log 2>&1',
      'rm *.log > /dev/null',
      `rm -rf ${path.join(project, 'ui/dist')}`,
      'find . -name "*.tmp" -delete',
      'git commit -m "rm stale files"',
      'cd ui && rm -rf dist',
    ]) {
      assert.equal(bash(command), null, command);
    }
  });

  test('asks before deleting existing tests or unresolvable targets', () => {
    for (const command of [
      'rm ui/src/app.test.ts',
      'rm -rf core/tests',
      'rm -rf "$UNKNOWN_VAR_FOR_TEST/x"',
      'git ls-files | xargs rm',
      'rm -rf *',
      'find . -delete',
      'rm',
    ]) {
      assert.equal(bash(command)?.permissionDecision, 'ask', command);
    }
  });

  test('PowerShell and Git Bash paths on Windows', () => {
    assert.equal(powershell('Remove-Item -Recurse -Force C:\\Users\\dev\\Documents')?.permissionDecision, 'deny');
    assert.equal(powershell('Remove-Item -Path ..\\other -Recurse')?.permissionDecision, 'deny');
    assert.equal(powershell('Remove-Item -LiteralPath .\\ui\\dist -Recurse -Force'), null);
    assert.equal(powershell('rmdir /s /q build'), null);
    const gitBash = runHook('guard-delete.mjs', preToolUse('Bash', { command: 'rm -rf /c/Users/dev' }, WINDOWS_PROJECT), {
      projectDir: WINDOWS_PROJECT,
    });
    assert.equal(gitBash?.permissionDecision, 'deny');
  });
});

describe('broken input never blocks', () => {
  for (const hook of ['protect-inputs.mjs', 'guard-tests.mjs', 'format-on-edit.mjs', 'guard-delete.mjs']) {
    test(hook, () => {
      for (const stdin of ['', 'not json', '[]', '{"tool_input": null}', '{"tool_input": {"file_path": 42, "command": 7}}']) {
        assert.equal(runHook(hook, stdin), null, JSON.stringify(stdin));
      }
    });
  }
});
