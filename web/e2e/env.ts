/** Env shared by the e2e engine, the CLI calls and the Next server. Test-only values. */
// Not OUTBOX_DIR: an exported dev .env sets that to ./outbox, which global setup would wipe.
export const OUTBOX = process.env.E2E_OUTBOX_DIR ?? "/tmp/mailrust-e2e-outbox";
export const WEB_SECRET = "e2e-web-secret-0123456789abcdef0123";

export const ENGINE_ENV: NodeJS.ProcessEnv = {
  ...process.env,
  MAILRUST_SENDER: "fake",
  OUTBOX_DIR: OUTBOX,
  PUBLIC_URL: "http://localhost:8080",
  WEB_URL: "http://localhost:3000",
  BIND_ADDR: "127.0.0.1:8080",
  TRACKING_SECRET: "e2e-tracking-secret-0123456789abcdef",
  WEB_SECRET,
  SQLX_OFFLINE: "true",
};
