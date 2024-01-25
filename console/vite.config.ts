/// <reference types="vitest" />
/// <reference types="vite/client"/>

import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import svgr from 'vite-plugin-svgr';

export default defineConfig({
  plugins: [react(), svgr()],
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: './setupTest.ts',
  },
  build: {
    outDir: 'out',
  },
  server: {
    proxy: {
      '/api': {
        target: 'http://localhost:7777/',
        changeOrigin: true,
        secure: false,
      },
    },
  },
});
