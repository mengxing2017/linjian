import { defineConfig } from 'vitest/config'
import react from '@vitejs/plugin-react'
import { apiContractAliases, apiContractRoot, frontendRoot } from '../../tools/api-contract-aliases'

const apiOrigin = process.env.LINJIAN_API_ORIGIN ?? 'http://127.0.0.1:8080'

export default defineConfig({
  plugins: [react()],
  resolve: { alias: apiContractAliases },
  optimizeDeps: { exclude: ['@linjian/api-contract'] },
  server: {
    host: true, port: 5173, strictPort: true,
    fs: { allow: [frontendRoot, apiContractRoot] },
    proxy: Object.fromEntries(['/api', '/healthz', '/readyz'].map(path => [path, {
      target: apiOrigin, changeOrigin: true,
    }])),
  },
  test: {
    // Ant Design form interactions can exceed 20s on constrained build hosts.
    testTimeout: 60000,
    // Run JSDOM/Ant Design suites serially: concurrent form-heavy files exceed
    // reasonable per-test waits on constrained CI hosts even when isolated checks pass.
    maxWorkers: 1,
    fileParallelism: false,
    setupFiles: ['./src/test/setup.ts'],
    environment: 'jsdom',
    clearMocks: true,
  },
})
