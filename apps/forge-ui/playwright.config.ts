import { defineConfig } from '@playwright/test';
export default defineConfig({ testDir: './tests', fullyParallel: false, use: { channel: 'chrome', headless: true, viewport: { width: 1440, height: 960 } }, timeout: 30000 });
