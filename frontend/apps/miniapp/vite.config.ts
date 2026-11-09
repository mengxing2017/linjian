import { createRequire } from 'node:module'
import { defineConfig } from 'vite'
import { apiContractAliases, apiContractRoot, frontendRoot } from '../../tools/api-contract-aliases'
// DCloud publishes this plugin as CommonJS with a default export.
process.env.CI ??= 'true'
const uni = createRequire(import.meta.url)('@dcloudio/vite-plugin-uni').default
export default defineConfig({
  plugins: [uni()],
  resolve: { alias: apiContractAliases },
  server: { fs: { allow: [frontendRoot, apiContractRoot] } },
})
