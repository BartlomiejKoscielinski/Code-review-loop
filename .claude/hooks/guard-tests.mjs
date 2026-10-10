// PreToolUse (Edit|Write|MultiEdit): changing an existing test needs human approval.
// New test files pass without a prompt. Rust unit tests live inline in `#[cfg(test)]`
// modules, so for .rs files an edit after that marker also counts (heuristic: the
// test module is assumed to sit at the end of the file).

import fs from 'node:fs';
import { decide, isTestFile, pathApi, projectRoot, relativeInside, runHook } from './_lib.mjs';

const RUST_TEST_MARKER = /#\[cfg\(test\)\]/;

runHook((input) => {
  const toolInput = input.tool_input ?? {};
  const filePath = toolInput.file_path;
  if (typeof filePath !== 'string' || filePath === '') return;

  const root = projectRoot(input);
  const rel = relativeInside(root, filePath);
  if (rel === null) return;

  let current;
  try {
    current = fs.readFileSync(pathApi(root, filePath).resolve(root, filePath), 'utf8');
  } catch {
    return; // new file: nothing to weaken
  }

  if (isTestFile(rel)) {
    decide(
      'ask',
      `${rel} to istniejący plik testów. Zasada projektu: istniejących testów nie zmieniamy ani nie osłabiamy ` +
        'bez zgody człowieka (.claude/rules/tests.md). Zatwierdź tylko, jeśli zmiana jest uzgodniona.',
    );
    return;
  }

  if (rel.toLowerCase().endsWith('.rs') && touchesRustTestModule(current, input.tool_name, toolInput)) {
    decide(
      'ask',
      `Zmiana w ${rel} dotyka modułu #[cfg(test)], czyli istniejących testów. ` +
        'Zasada projektu: edycja istniejących testów wymaga zgody człowieka (.claude/rules/tests.md).',
    );
  }
});

function touchesRustTestModule(current, toolName, toolInput) {
  const content = normalize(current);
  const marker = content.search(RUST_TEST_MARKER);
  if (marker === -1) return false;

  if (toolName === 'Write') {
    const next = normalize(toolInput.content);
    const nextMarker = next.search(RUST_TEST_MARKER);
    return content.slice(marker) !== (nextMarker === -1 ? '' : next.slice(nextMarker));
  }

  const edits = Array.isArray(toolInput.edits) ? toolInput.edits : [toolInput];
  return edits.some((edit) => {
    const old = normalize(edit?.old_string);
    if (old === '') return false;
    if (/#\[(?:cfg\(test\)|test)\]/.test(old)) return true;
    const position = edit.replace_all ? content.lastIndexOf(old) : content.indexOf(old);
    return position >= marker;
  });
}

function normalize(text) {
  return typeof text === 'string' ? text.replace(/\r\n/g, '\n') : '';
}
