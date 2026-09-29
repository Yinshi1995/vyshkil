import { defineConfig } from "@playwright/test";

// Сервер (`cargo leptos build` + `server.exe`) запускається окремо перед тестами -- не тут:
// підняти PostgreSQL-залежний Axum-сервер через `webServer` тут не варте свічок,
// простіше й надійніше піднімати вручну/в CI-кроці (docs/spec/06-roadmap.md, Етап 4).
export default defineConfig({
  testDir: "./tests",
  timeout: 30_000,
  fullyParallel: false,
  retries: 0,
  use: {
    baseURL: process.env.BASE_URL ?? "http://localhost:3000",
    trace: "retain-on-failure",
  },
});
