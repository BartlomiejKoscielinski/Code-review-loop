import { invoke, type InvokeArgs } from '@tauri-apps/api/core';
import type { z } from 'zod';
import type { AppInfo } from './generated/AppInfo';
import type { ErrorCode } from './generated/ErrorCode';
import type { IpcError } from './generated/IpcError';
import { appInfoSchema, ipcErrorSchema } from './schemas';

export type { AppInfo, ErrorCode, IpcError };

/** Errors raised on the UI side of the boundary, not by the Rust core. */
export type ClientError = {
  code: 'invalid_response' | 'unavailable';
  message: string;
};

export type CallError = IpcError | ClientError;

export type CallResult<T> = { ok: true; value: T } | { ok: false; error: CallError };

/**
 * Calls a Rust command and validates its response. Never throws: failures
 * come back as `{ ok: false }`, so components can render them.
 */
export async function call<T>(
  command: string,
  schema: z.ZodType<T>,
  args?: InvokeArgs,
): Promise<CallResult<T>> {
  let raw: unknown;
  try {
    raw = await invoke(command, args);
  } catch (thrown) {
    const coreError = ipcErrorSchema.safeParse(thrown);
    if (coreError.success) return { ok: false, error: coreError.data };
    // Not an IpcError: no Tauri runtime, a command missing from the capability,
    // or a serialization failure. Details stay in the dev console only.
    if (import.meta.env.DEV) console.error(`IPC ${command} failed`, thrown);
    return {
      ok: false,
      error: {
        code: 'unavailable',
        message: `Command ${command} failed outside the core contract`,
      },
    };
  }
  const parsed = schema.safeParse(raw);
  if (parsed.success) return { ok: true, value: parsed.data };
  return {
    ok: false,
    error: { code: 'invalid_response', message: `Unexpected response from ${command}` },
  };
}

export function appInfo(): Promise<CallResult<AppInfo>> {
  return call('app_info', appInfoSchema);
}
