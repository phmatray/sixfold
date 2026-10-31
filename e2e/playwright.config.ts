import { defineConfig, devices } from "@playwright/test";

// The journeys run against a release build in ../web/dist (`trunk build --release`),
// served as static files. Every test gets a fresh browser context (empty storage),
// and the service worker is blocked so nothing is served from an old cache.
// PW_PORT picks another port, so two checkouts can run their journeys side by side.
const PORT = Number(process.env.PW_PORT ?? 4173);

export default defineConfig({
  testDir: ".",
  timeout: 180_000,
  expect: { timeout: 15_000 },
  retries: process.env.CI ? 2 : 0,
  workers: process.env.CI ? 2 : 1,
  reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : "list",
  use: {
    baseURL: `http://127.0.0.1:${PORT}`,
    locale: "en-US",
    reducedMotion: "reduce",
    serviceWorkers: "block",
    actionTimeout: 15_000,
    trace: "on-first-retry",
    // WebGL for the three.js world in headless Chromium.
    launchOptions: { args: ["--enable-unsafe-swiftshader", "--use-angle=swiftshader"] },
  },
  projects: [
    { name: "desktop", use: { ...devices["Desktop Chrome"] } },
    // 390×844 with touch (iPhone-sized), Chromium: the journeys tagged @phone.
    {
      name: "phone",
      use: { ...devices["Desktop Chrome"], viewport: { width: 390, height: 844 }, isMobile: true, hasTouch: true },
      grep: /@phone/,
    },
    // The engine Safari and the macOS desktop app draw with: the journeys tagged @webkit
    // (PW_WEBKIT_ALL=1: every journey, before a release).
    { name: "webkit", use: { ...devices["Desktop Safari"] }, grep: process.env.PW_WEBKIT_ALL ? undefined : /@webkit/ },
  ],
  webServer: {
    command: `node serve.mjs ${PORT}`,
    url: `http://127.0.0.1:${PORT}/index.html`,
    reuseExistingServer: !process.env.CI,
  },
});
