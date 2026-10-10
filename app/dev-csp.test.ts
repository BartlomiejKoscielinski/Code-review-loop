import { describe, expect, it } from 'vitest';
import { DEV_CSP } from './dev-csp';

const LOCAL_SOURCES = new Set([
  "'self'",
  "'none'",
  "'unsafe-inline'",
  'data:',
  'ipc:',
  'http://ipc.localhost',
  'ws://localhost:1420',
]);

describe('DEV_CSP', () => {
  it('allows only local sources, so dev builds cannot load or send to remote hosts', () => {
    const sources = DEV_CSP.split(';').flatMap((directive) =>
      directive.trim().split(/\s+/).slice(1),
    );
    expect(sources.filter((source) => !LOCAL_SOURCES.has(source))).toEqual([]);
  });

  it('keeps unsafe-inline out of every directive except scripts and styles', () => {
    const withInline = DEV_CSP.split(';')
      .map((directive) => directive.trim())
      .filter((directive) => directive.includes("'unsafe-inline'"))
      .map((directive) => directive.split(' ')[0]);
    expect(withInline).toEqual(['script-src', 'style-src']);
  });
});
