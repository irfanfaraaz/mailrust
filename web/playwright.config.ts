import { defineConfig } from "@playwright/test";
import { WEB_SECRET } from "./e2e/env";

export default defineConfig({
  testDir: "./e2e",
  // Seeds the database, then starts the engine (it needs the seeded SYSTEM_* values).
  globalSetup: "./e2e/global-setup.ts",
  fullyParallel: false,
  workers: 1,
  use: { baseURL: "http://localhost:3000" },
  webServer: {
    command: "pnpm build && pnpm start",
    url: "http://localhost:3000/login",
    env: { ENGINE_URL: "http://127.0.0.1:8080", WEB_SECRET },
    timeout: 300_000,
    reuseExistingServer: !process.env.CI,
  },
});
