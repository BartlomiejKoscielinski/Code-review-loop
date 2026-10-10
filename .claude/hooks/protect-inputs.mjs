// PreToolUse (Edit|Write|MultiEdit|NotebookEdit): docs/input/ is read-only for the agent.
// Backed by the `Edit(/docs/input/**)` deny rule in .claude/settings.json.

import { decide, isUnder, projectRoot, relativeInside, runHook } from './_lib.mjs';

runHook((input) => {
  const toolInput = input.tool_input ?? {};
  const target = toolInput.file_path ?? toolInput.notebook_path;
  if (typeof target !== 'string' || target === '') return;

  const rel = relativeInside(projectRoot(input), target);
  if (rel === null || !isUnder(rel, 'docs/input')) return;

  decide(
    'deny',
    `Zapis do ${rel} zablokowany: docs/input/ zawiera materiały wejściowe zespołu i agent tylko je czyta ` +
      '(CLAUDE.md, docs/input/README.md). Rozbieżności opisz w docs/plan/ albo w ADR i zgłoś człowiekowi.',
  );
});
