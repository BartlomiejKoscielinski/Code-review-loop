// PreToolUse (Bash|PowerShell): blocks deleting files outside the repository.
// Permission rules can't express "outside the repo" and the sandbox doesn't run on
// native Windows, so this hook resolves delete targets itself.
//   deny: target outside the repo, the repo root, .git/, docs/input/
//   ask:  existing test files or test directories, targets it can't resolve
// Best effort: commands hidden in $(...), backticks, scripts or aliases are not seen.

import path from 'node:path';
import {
  decide,
  exists,
  homeDir,
  isTestFile,
  isUnder,
  pathApi,
  projectRoot,
  relativeInside,
  runHook,
} from './_lib.mjs';

const DELETE_COMMANDS = new Set(['rm', 'rmdir', 'unlink', 'shred', 'del', 'erase', 'rd', 'ri', 'remove-item']);
const CMD_STYLE = new Set(['del', 'erase', 'rd', 'rmdir']); // `/s /q` are flags, not paths
const WRAPPERS = new Set(['sudo', 'command', 'builtin', 'time', 'nice', 'nohup', 'env', 'exec']);
const SHELLS = new Set(['sh', 'bash', 'zsh', 'cmd', 'cmd.exe', 'pwsh', 'pwsh.exe', 'powershell', 'powershell.exe']);
const PS_PATH_PARAMS = new Set(['-path', '-literalpath', '-lp']);
const PS_VALUE_PARAMS = new Set(['-include', '-exclude', '-filter', '-credential', '-stream']);
const FIND_FILTERS = new Set([
  '-name', '-iname', '-path', '-ipath', '-regex', '-iregex', '-type', '-newer', '-mtime', '-mmin', '-size', '-empty',
]);
const REDIRECT =/^(?:\d|\*|&)?(?:>>?|<)&?/;
const QUICK_FILTER = /\b(?:rm|rmdir|unlink|shred|del|erase|rd|ri|remove-item|find)\b/i;
const MAX_NESTING = 3;

runHook((input) => {
  const command = input.tool_input?.command;
  if (typeof command !== 'string' || !QUICK_FILTER.test(command)) return;

  const root = projectRoot(input);
  const findings = [];
  inspect(command, input.tool_name === 'PowerShell', input.cwd || root, { root, findings }, 0);

  const reasons = (decision) => [...new Set(findings.filter((f) => f.decision === decision).map((f) => f.reason))];
  const denied = reasons('deny');
  const asked = reasons('ask');
  if (denied.length > 0) decide('deny', `${denied.join(' ')} Usuwaj tylko pliki w repo, poza .git/ i docs/input/.`);
  else if (asked.length > 0) decide('ask', asked.join(' '));
});

function inspect(command, powershell, startCwd, context, depth) {
  let cwd = startCwd;
  for (const segment of splitSegments(command, powershell)) {
    const words = stripWrappers(tokenize(segment, powershell));
    if (words.length === 0) continue;
    const name = words[0].toLowerCase().replace(/^.*[\\/]/, '');
    const args = words.slice(1);

    if (['cd', 'pushd', 'set-location', 'sl', 'chdir'].includes(name)) {
      const target = args.find((w) => !w.startsWith('-'));
      cwd = (target && expand(target, context.root, cwd)?.resolved) || cwd;
    } else if (SHELLS.has(name)) {
      const flag = args.findIndex((w) => /^(?:-c|-command|\/c|\/k)$/i.test(w));
      if (flag !== -1 && flag + 1 < args.length && depth < MAX_NESTING) {
        const nestedPowerShell = name.startsWith('pwsh') || name.startsWith('powershell');
        inspect(args.slice(flag + 1).join(' '), nestedPowerShell, cwd, context, depth + 1);
      }
    } else if (name === 'xargs') {
      if (args.some((w) => DELETE_COMMANDS.has(w.toLowerCase()))) {
        add(context, 'ask', 'Usuwanie przez xargs: nie da się ustalić, które pliki zostaną usunięte.');
      }
    } else if (name === 'find') {
      const deletes =
        args.includes('-delete') ||
        args.some((w, i) => (w === '-exec' || w === '-execdir') && DELETE_COMMANDS.has((args[i + 1] ?? '').toLowerCase()));
      if (deletes) {
        const end = args.findIndex((w) => w.startsWith('-') || w === '(' || w === '!');
        const starts = args.slice(0, end === -1 ? args.length : end);
        // Start points are search roots: with a filter only matches go, without one everything.
        const filtered = args.some((w) => FIND_FILTERS.has(w));
        checkTargets(starts.length > 0 ? starts : ['.'], cwd, context, filtered ? '**' : '*');
      }
    } else if (DELETE_COMMANDS.has(name)) {
      const targets = deleteTargets(name, args, powershell);
      if (targets.length === 0) add(context, 'ask', `Nie da się ustalić, co usuwa polecenie „${segment.trim()}”.`);
      else checkTargets(targets, cwd, context);
    }
  }
}

function deleteTargets(name, args, powershell) {
  const targets = [];
  const split = (value) => (powershell ? value.split(',') : [value]);
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    const lower = arg.toLowerCase();
    if (arg === '--') {
      targets.push(...args.slice(i + 1));
      break;
    }
    if (powershell && PS_PATH_PARAMS.has(lower)) {
      if (i + 1 < args.length) targets.push(...split(args[++i]));
    } else if (powershell && PS_VALUE_PARAMS.has(lower)) {
      i++;
    } else if (!arg.startsWith('-') && !(CMD_STYLE.has(name) && /^\/[a-z]$/i.test(arg))) {
      targets.push(...split(arg));
    }
  }
  return targets.filter((target) => target !== '');
}

function checkTargets(targets, cwd, context, searchPattern = null) {
  for (const target of targets) {
    const expanded = expand(target, context.root, cwd);
    if (!expanded) {
      add(context, 'ask', `Nie da się ustalić ścieżki „${target}” (zmienne lub podstawienia).`);
      continue;
    }
    const { resolved } = expanded;
    const pattern = searchPattern ?? expanded.pattern;
    const rel = relativeInside(context.root, resolved);
    if (rel === null) add(context, 'deny', `„${target}” wskazuje poza repo (${resolved}).`);
    else if (rel === '' && pattern === null) add(context, 'deny', `„${target}” usuwa cały katalog repo.`);
    else if (rel === '' && /^(?:\.?\*|\*\.\*)$/.test(pattern)) add(context, 'ask', `„${target}” usuwa całą zawartość repo.`);
    else if (isUnder(rel, '.git')) add(context, 'deny', `„${target}” usuwa dane gita (.git/).`);
    else if (isUnder(rel, 'docs/input')) add(context, 'deny', `„${target}” leży w docs/input/, który agent tylko czyta.`);
    else if ((isTestFile(rel) || isTestFile(`${rel}/_`)) && exists(resolved)) {
      add(context, 'ask', `„${target}” to istniejące testy; ich usunięcie wymaga zgody (.claude/rules/tests.md).`);
    }
  }
}

function add(context, decision, reason) {
  context.findings.push({ decision, reason });
}

/**
 * `{ resolved, pattern }` for a delete target: the absolute path (for globs, the directory
 * before the first glob) and the glob remainder or null. Null when variables are unknown.
 */
function expand(target, root, cwd) {
  let value = target.replace(/\$\{?(?:env:)?(\w+)\}?|%(\w+)%/gi, (match, unix, windows) => {
    const name = (unix ?? windows).toUpperCase();
    if (name === 'HOME' || name === 'USERPROFILE') return homeDir;
    if (name === 'PWD') return cwd;
    if (name === 'CLAUDE_PROJECT_DIR') return root;
    const key = Object.keys(process.env).find((k) => k.toUpperCase() === name);
    return key ? process.env[key] : match;
  });
  if (/[$%`]/.test(value)) return null;
  if (value === '~' || value.startsWith('~/') || value.startsWith('~\\')) value = homeDir + value.slice(1);

  const api = pathApi(root, cwd, value);
  // Git Bash spelling of Windows drives: /c/Users -> c:/Users
  if (api === path.win32) value = value.replace(/^\/([a-zA-Z])(?=\/|$)/, '$1:/');

  // Check the part before the first glob: `/tmp/*` checks `/tmp/`, `*.log` checks cwd.
  const glob = value.search(/[*?[]/);
  if (glob === -1) return { resolved: api.resolve(cwd, value), pattern: null };
  const separator = Math.max(value.lastIndexOf('/', glob), value.lastIndexOf('\\', glob));
  return {
    resolved: api.resolve(cwd, separator === -1 ? '.' : value.slice(0, separator + 1)),
    pattern: value.slice(separator + 1),
  };
}

/** Splits on ; | & && || and newlines outside quotes. `2>&1` and `&>` stay intact. */
function splitSegments(command, powershell) {
  const segments = [];
  let current = '';
  let quote = null;
  for (let i = 0; i < command.length; i++) {
    const char = command[i];
    if (quote) {
      current += char;
      if (char === '\\' && quote === '"' && !powershell && i + 1 < command.length) current += command[++i];
      else if (char === quote) quote = null;
    } else if (char === '"' || char === "'") {
      quote = char;
      current += char;
    } else if (char === '\\' && !powershell && i + 1 < command.length) {
      current += char + command[++i];
    } else if (char === '&' && (command[i - 1] === '>' || command[i + 1] === '>')) {
      current += char;
    } else if (';|&\n'.includes(char)) {
      segments.push(current);
      current = '';
    } else {
      current += char;
    }
  }
  segments.push(current);
  return segments.filter((segment) => segment.trim() !== '');
}

/** Shell-like words without quotes. Redirections and their targets are dropped. */
function tokenize(segment, powershell) {
  const words = [];
  let current = '';
  let started = false;
  let quote = null;
  const flush = () => {
    if (started) words.push(current);
    current = '';
    started = false;
  };
  for (let i = 0; i < segment.length; i++) {
    const char = segment[i];
    if (quote) {
      if (char === quote) quote = null;
      else if (char === '\\' && quote === '"' && !powershell && '"\\$`'.includes(segment[i + 1] ?? '')) current += segment[++i];
      else current += char;
    } else if (char === '"' || char === "'") {
      quote = char;
      started = true;
    } else if (char === '\\' && !powershell) {
      current += segment[++i] ?? '';
      started = true;
    } else if (/\s/.test(char)) {
      flush();
    } else {
      current += char;
      started = true;
    }
  }
  flush();

  const result = [];
  for (let i = 0; i < words.length; i++) {
    const match = REDIRECT.exec(words[i]);
    if (!match) result.push(words[i]);
    else if (match[0] === words[i] && !words[i].endsWith('&')) i++; // operator alone: skip its target too
  }
  return result;
}

function stripWrappers(words) {
  let i = 0;
  while (i < words.length) {
    const word = words[i].toLowerCase();
    if (/^\w+=/.test(words[i])) i++;
    else if (word === 'timeout') i += 2;
    else if (WRAPPERS.has(word)) i++;
    else break;
  }
  return words.slice(i);
}
