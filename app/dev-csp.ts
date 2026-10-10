// CSP for `npm run app:dev`. Tauri applies `app.security.csp` only to pages
// served by its own protocol; in dev the window loads the Vite server
// directly, so Vite sends this policy as a header. It differs from production
// only where Vite needs it: inline scripts and styles (React Refresh, HMR) and
// the HMR websocket.
export const DEV_CSP = [
  "default-src 'self'",
  "script-src 'self' 'unsafe-inline'",
  "style-src 'self' 'unsafe-inline'",
  "img-src 'self' data:",
  "font-src 'self'",
  "connect-src 'self' ws://localhost:1420 ipc: http://ipc.localhost",
  "object-src 'none'",
  "base-uri 'self'",
  "form-action 'none'",
  "frame-ancestors 'none'",
].join('; ');
