import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';
import { DEV_CSP } from './dev-csp.ts';

// Tauri expects a fixed dev port and serves the build output from `dist/`.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true, headers: { 'Content-Security-Policy': DEV_CSP } },
  build: { target: 'es2022' },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test-setup.ts'],
  },
});
