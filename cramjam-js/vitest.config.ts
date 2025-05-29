import wasm from 'vite-plugin-wasm';
// import topLevelAwait from 'vite-plugin-top-level-await';
import {defineConfig} from 'vitest/config';

export default defineConfig({
  plugins: [wasm()],
  test: {
    environment: 'jsdom', // or 'node' depending on your WASM usage
    include: ['tests/**/*.test.ts'],
    setupFiles: ['./tests/setup.ts'], // Optional init
  },
});
