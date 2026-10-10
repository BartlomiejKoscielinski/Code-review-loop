import { defineConfig } from 'vitest/config';

// mockIPC from @tauri-apps/api needs a window object.
export default defineConfig({
  test: { environment: 'jsdom' },
});
