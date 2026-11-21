import { test } from 'node:test'
import assert from 'node:assert/strict'
import { dependencyCycles, imports, violations } from './check-boundaries.mjs'
test('contract consumers cannot bypass package exports through deep imports', () => {
  const file = 'apps/web/src/features/community/api/index.ts'
  for (const specifier of [
    '@linjian/api-contract/community/client',
    '@linjian/api-contract/src/modules/community/types',
    '@linjian/api-contract/http/browser-client',
    '@linjian/api-contract',
  ])
    assert.ok(
      violations(file, `export * from '${specifier}'`).some((error) =>
        error.includes('禁止深层导入'),
      ),
    )
  for (const specifier of ['community', 'http', 'client', 'miniapp-users/browser'])
    assert.deepEqual(violations(file, `export * from '@linjian/api-contract/${specifier}'`), [])
})
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

test('all app, feature API and shared package indexes only expose declarations', () => {
  for (const file of [
    'apps/web/src/features/example/index.tsx',
    'apps/miniapp/src/features/media/api/index.ts',
    'apps/web/src/app/api/index.ts',
    'apps/miniapp/src/index.ts',
    'packages/china-regions/src/index.ts',
  ]) {
    for (const source of [
      "export { Page } from './page'",
      "export { default } from './page'",
      'export type Value = string',
      'export interface Value {}',
    ])
      assert.deepEqual(violations(file, source), [])
    for (const source of [
      'export default function Entry() { return <SessionGate>{user => <Page user={user} />}</SessionGate> }',
      'export function Page() { return <View /> }',
      'export async function save() { return api.save() }',
      'export const client = createClient(transport)',
      'const state = {}',
      'let current = 1',
      'class Client {}',
      'createClient(transport)',
      'export default <View />',
      'export default client',
      "import './initialize'",
    ])
      assert.ok(violations(file, source).length, `${file}: ${source}`)
  }
})

test('contract access is limited to named feature API bridges and pure indexes', () => {
  for (const file of [
    'apps/miniapp/src/features/media/api/client.ts',
    'apps/web/src/features/admin/statistics/api/contract.ts',
  ])
    assert.deepEqual(
      violations(file, "import { createClient } from '@linjian/api-contract/client'"),
      [],
    )
  for (const file of [
    'apps/miniapp/src/app/api/client.ts',
    'apps/web/src/features/community/page.tsx',
    'apps/miniapp/src/features/media/api/types.ts',
  ])
    assert.ok(violations(file, "import type { Client } from '@linjian/api-contract/client'").length)
})

test('raw client and transport creation stay in the application composition root', () => {
  for (const source of [
    'export const client = createClient(transport)',
    'const transport = createUniTransport(base)',
  ]) {
    assert.deepEqual(violations('apps/miniapp/src/app/api/client.ts', source), [])
    for (const file of [
      'apps/miniapp/src/features/identity/api/client.ts',
      'apps/miniapp/src/shared/client.ts',
      'apps/web/src/features/admin/operations/api/client.ts',
    ])
      assert.ok(
        violations(file, source).some((error) => error.includes('只能在 app/api/client.ts')),
      )
  }
})

test('feature API implementations cannot be imported by another feature', () => {
  const file = 'apps/miniapp/src/features/community/api/client.ts'
  for (const source of [
    "import { identityApi } from '../../identity/api/client'",
    "export { identityApi } from '../../identity/api/client.ts'",
    "const load = () => import('../../identity/api/client')",
    "type Value = import('../../identity/api/client').Value",
  ])
    assert.ok(violations(file, source).some((error) => error.includes('禁止跨 feature')))
  assert.deepEqual(violations(file, "import { client } from '../../../app/api'"), [])
  assert.deepEqual(
    violations(
      'apps/miniapp/src/features/identity/api/index.ts',
      "export { identityApi } from './client'",
    ),
    [],
  )
  assert.deepEqual(violations(file, "import { identityApi } from '../../identity/api'"), [])
  assert.ok(
    violations(
      'apps/web/src/features/admin/operations/api/client.ts',
      "import { statisticsApi } from '../../statistics/api/client'",
    ).length,
  )
  for (const source of [
    "import { client } from '../../identity/api'",
    "import { client as appClient } from '../../identity/api/index.ts'",
  ])
    assert.ok(
      violations(file, source).some((error) => error.includes('共享 client 必须来自 app/api')),
    )
  assert.ok(
    violations(
      'apps/miniapp/src/features/community/Page.vue',
      '<script setup lang="ts">import { client } from "../identity/api"</script>',
    ).some((error) => error.includes('共享 client 必须来自 app/api')),
  )
})

test('detects runtime cycles across TS, TSX, Vue scripts, directory entries and packages', () => {
  assert.deepEqual(
    dependencyCycles(
      new Map([
        ['features/a/client.ts', "import { value } from '../b'"],
        ['features/b/index.ts', "export { value } from './page.vue'"],
        [
          'features/b/page.vue',
          '<script setup lang="ts">const load = () => import("../a/client")</script>',
        ],
      ]),
    ),
    [
      [
        'features/a/client.ts',
        'features/b/index.ts',
        'features/b/page.vue',
        'features/a/client.ts',
      ],
    ],
  )
  assert.deepEqual(
    dependencyCycles(
      new Map([
        ['packages/china-regions/src/index.ts', "export { value } from './data'"],
        ['packages/china-regions/src/data.ts', "import { value } from '@linjian/china-regions'"],
      ]),
    ),
    [
      [
        'packages/china-regions/src/index.ts',
        'packages/china-regions/src/data.ts',
        'packages/china-regions/src/index.ts',
      ],
    ],
  )
  for (const source of [
    "import type { Value } from './client'",
    "import { type Value } from './client'",
    "export type { Value } from './client'",
    "export { type Value } from './client'",
    "type Value = import('./client').Value",
  ])
    assert.deepEqual(imports(source, 'types.ts', true), [])
  assert.deepEqual(
    dependencyCycles(
      new Map([
        ['client.ts', "import type { Value } from './types'"],
        ['types.ts', "import { client } from './client'"],
      ]),
    ),
    [],
  )
})

test('requires row action columns to stay fixed on the right', () => {
  const file = 'apps/web/src/features/admin/example.tsx'
  assert.equal(violations(file, "const column={title:'操作',width:120,render:()=>null}").length, 1)
  assert.equal(
    violations(file, "const column={title:'操作',width:120,fixed:'right',render:()=>null}").length,
    0,
  )
  assert.equal(
    violations(file, "const column={title:'操作',width:120,fixed:'right' as const,render:()=>null}")
      .length,
    0,
  )
  assert.equal(
    violations(
      'apps/web/src/shared/row-actions.tsx',
      "const column={title:'操作',width:120,render:()=>null}",
    ).length,
    0,
  )
})
