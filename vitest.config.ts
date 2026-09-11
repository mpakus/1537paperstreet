import { svelte } from '@sveltejs/vite-plugin-svelte'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  plugins: [svelte({ configFile: 'ui/svelte.config.js' })],
  test: {
    include: ['ui/src/**/*.test.ts'],
    environmentMatchGlobs: [
      ['ui/src/lib/find.test.ts', 'happy-dom'],
      ['ui/src/lib/tree.dom.test.ts', 'happy-dom'],
    ],
    coverage: {
      provider: 'v8',
      include: ['ui/src/lib/tree.ts', 'ui/src/lib/text.ts'],
      exclude: ['ui/src/**/*.test.ts'],
      reporter: ['text', 'html', 'json'],
      reportsDirectory: 'coverage/ui',
      thresholds: {
        lines: 70,
        statements: 70,
      },
    },
  },
})
