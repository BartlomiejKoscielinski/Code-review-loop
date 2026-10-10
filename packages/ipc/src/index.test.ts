import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { z } from 'zod';
import { appInfo, call } from './index';

afterEach(() => {
  clearMocks();
});

const numberSchema = z.object({ value: z.number() });

describe('call', () => {
  it('returns the parsed value when the response matches the schema', async () => {
    mockIPC(() => ({ value: 42 }));
    await expect(call('answer', numberSchema)).resolves.toEqual({
      ok: true,
      value: { value: 42 },
    });
  });

  it('reports invalid_response when the response does not match the schema', async () => {
    mockIPC(() => ({ value: 'not a number' }));
    const result = await call('answer', numberSchema);
    expect(result).toEqual({
      ok: false,
      error: { code: 'invalid_response', message: 'Unexpected response from answer' },
    });
  });

  it('passes a core IpcError through unchanged', async () => {
    mockIPC(() => {
      throw { code: 'invalid_input', message: 'bad path' };
    });
    await expect(call('answer', numberSchema)).resolves.toEqual({
      ok: false,
      error: { code: 'invalid_input', message: 'bad path' },
    });
  });

  it('reports unavailable when the failure is not an IpcError', async () => {
    mockIPC(() => {
      throw new Error('boom');
    });
    const result = await call('answer', numberSchema);
    expect(result).toEqual({
      ok: false,
      error: { code: 'unavailable', message: 'Command answer failed outside the core contract' },
    });
  });
});

describe('appInfo', () => {
  it('calls the app_info command', async () => {
    const commands: string[] = [];
    mockIPC((command) => {
      commands.push(command);
      return { name: 'Lupa', version: '0.1.0' };
    });
    await expect(appInfo()).resolves.toEqual({
      ok: true,
      value: { name: 'Lupa', version: '0.1.0' },
    });
    expect(commands).toEqual(['app_info']);
  });
});
