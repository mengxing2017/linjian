import assert from 'node:assert/strict'
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { test } from 'node:test'
import { check, cycles, dependencies, entryViolations } from './check-boundaries.mjs'

test('entries expose only explicit exports', () => {
  for (const file of ['src/modules/community/index.ts', 'src/http/index.ts']) {
    assert.deepEqual(
      entryViolations(
        file,
        "export { createClient } from './client'\nexport type { Client } from './client'",
      ),
      [],
    )
    for (const source of [
      'export const client = createClient()',
      'export function createClient() {}',
      'export class Client {}',
      'export interface Client {}',
      "import './default-client'",
      "export * from './client'",
      "export type * from './types'",
      'export default createClient()',
      'createClient()',
    ])
      assert.ok(entryViolations(file, source).length, source)
  }
  assert.deepEqual(
    entryViolations('src/modules/community/client.ts', 'export function createClient() {}'),
    [],
  )
})

test('dependency inspection covers static, dynamic and type imports', () => {
  assert.deepEqual(
    dependencies(
      'client.ts',
      `
    import type { Client } from './client'
    export { value } from './value'
    type Post = import('./types').Post
    const load = () => import('./lazy')
    const computed = () => import(variable)
  `,
    ),
    ['./client', './value', './types', './lazy', '<computed-import>'],
  )
})

test('cycle detection includes disconnected components and self references', () => {
  assert.deepEqual(
    cycles(
      new Map([
        ['client', ['types']],
        ['types', []],
      ]),
    ),
    [],
  )
  assert.deepEqual(
    cycles(
      new Map([
        ['entry', []],
        ['client', ['query']],
        ['query', ['client']],
      ]),
    ),
    ['client -> query -> client'],
  )
  assert.deepEqual(cycles(new Map([['self', ['self']]])), ['self -> self'])
})

test('named files cannot depend on index, including type and dynamic imports', () => {
  const directory = fs.mkdtempSync(path.join(os.tmpdir(), 'linjian-contract-boundaries-'))
  try {
    const module = path.join(directory, 'src/modules/example')
    fs.mkdirSync(module, { recursive: true })
    fs.writeFileSync(
      path.join(directory, 'package.json'),
      JSON.stringify({
        name: '@linjian/api-contract',
        exports: { './example': './src/modules/example/index.ts' },
      }),
    )
    fs.writeFileSync(path.join(module, 'index.ts'), "export { value } from './client'")
    for (const source of [
      "import { value } from './index'",
      "type Value = import('./index').Value",
      "const load = () => import('./index')",
      "export { value } from './index'",
      "import { value } from '@linjian/api-contract/example'",
    ]) {
      fs.writeFileSync(path.join(module, 'client.ts'), source)
      assert.throws(() => check(directory), /具名模块禁止依赖入口/)
    }
    fs.writeFileSync(path.join(module, 'client.ts'), 'export const value = 1')
    assert.equal(check(directory), 2)
  } finally {
    fs.rmSync(directory, { recursive: true, force: true })
  }
})

test('contract source has no reverse entry dependencies or cycles', () => {
  assert.ok(check() > 0)
})
