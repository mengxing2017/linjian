import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'

export const frontendRoot = fileURLToPath(new URL('../', import.meta.url))
const contractRoot = new URL('../../api-contract/', import.meta.url)
export const apiContractRoot = fileURLToPath(contractRoot)
const manifest = JSON.parse(
  readFileSync(new URL('package.json', contractRoot), 'utf8'),
) as { name: string; exports: Record<string, string> }

// Resolve public entry points to watched source, rather than pnpm's file: snapshot.
// Exact matches preserve the package's exports boundary and a single browser client.
export const apiContractAliases = Object.entries(manifest.exports).map(([entry, target]) => ({
  find: new RegExp(`^${(manifest.name + entry.slice(1)).replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}$`),
  replacement: fileURLToPath(new URL(target, contractRoot)),
}))
