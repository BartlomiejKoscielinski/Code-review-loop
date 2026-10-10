import { z } from 'zod';
import type { AppInfo } from './generated/AppInfo';
import type { ErrorCode } from './generated/ErrorCode';
import type { IpcError } from './generated/IpcError';

// Runtime schemas for values coming from the Rust core. The types in
// ./generated come from Rust (ts-rs); the checks at the bottom make
// `npm run typecheck` fail when a schema drifts from its generated type.

export const errorCodeSchema = z.enum(['internal', 'invalid_input']);

export const ipcErrorSchema = z.object({
  code: errorCodeSchema,
  message: z.string(),
});

export const appInfoSchema = z.object({
  name: z.string(),
  version: z.string(),
});

// Strict type equality: also catches differences in optional fields.
type Exact<A, B> =
  (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false;
type Assert<T extends true> = T;

export type ErrorCodeSchemaMatches = Assert<Exact<z.infer<typeof errorCodeSchema>, ErrorCode>>;
export type IpcErrorSchemaMatches = Assert<Exact<z.infer<typeof ipcErrorSchema>, IpcError>>;
export type AppInfoSchemaMatches = Assert<Exact<z.infer<typeof appInfoSchema>, AppInfo>>;
