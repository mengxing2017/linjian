import { test } from 'node:test'
import assert from 'node:assert/strict'
import { violations } from './check-boundaries.mjs'
test('checks static, type-only, re-export and dynamic imports, including Vue scripts', () => {
  for (const statement of [
    "import type {User} from '@linjian/api-contract/identity'",
    "export * from '@linjian/api-contract/community'",
    "const x=import('@linjian/api-contract/client')",
  ]) {
    assert.ok(
      violations(
        'apps/miniapp/src/pages/index.vue',
        `<script setup lang="ts">${statement}</script>`,
      ).length,
    )
  }
  assert.equal(
    violations(
      'apps/miniapp/src/features/identity/api/index.ts',
      "export type {User} from '@linjian/api-contract/identity'",
    ).length,
    0,
  )
  assert.ok(
    violations('apps/web/src/main.ts', "import x from '../../../../api-contract/src/http'").length,
  )
  assert.ok(violations('apps/web/src/main.ts', "import x from '../../miniapp/src/main'").length)
  assert.ok(violations('packages/core/index.ts', "export * from '../../apps/web/src/main'").length)
})

test('keeps public indexes as session wiring and compatibility exports', () => {
  const file = 'apps/web/src/features/example/index.tsx'
  for (const source of [
    "export { Page } from './page'",
    'export default function Entry() { return <SessionGate>{user => <Page user={user} />}</SessionGate> }',
  ])
    assert.equal(violations(file, source).length, 0)
  for (const source of [
    'export function Page() { const data = useQuery(); return <View data={data} /> }',
    'export async function save() { return api.save() }',
    'export const useSchools = () => useQuery({})',
  ])
    assert.ok(violations(file, source).length)
  assert.equal(
    violations(
      'apps/miniapp/src/features/media/api/index.ts',
      "import { createMediaClient } from '@linjian/api-contract/media'; export const mediaApi = createMediaClient(client)",
    ).length,
    0,
  )
})
