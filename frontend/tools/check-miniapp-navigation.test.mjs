import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import assert from 'node:assert/strict'
import vm from 'node:vm'
import { validateNavigation } from './check-miniapp-navigation.mjs'

const source = file => readFileSync(new URL(`../apps/miniapp/src/${file}`, import.meta.url), 'utf8')
const pages = JSON.parse(source('pages.json'))
test('source navigation contains exactly four root destinations, never a publish tab', () => {
  assert.doesNotThrow(() => validateNavigation(pages))
  assert.throws(() => validateNavigation({ ...pages, tabBar: { ...pages.tabBar, custom: false } }))
  assert.throws(() => validateNavigation({ ...pages, tabBar: { ...pages.tabBar, list: pages.tabBar.list.slice(0, 3) } }))
})
test('native component only bridges lifecycle and actions, without owning API calls', () => {
  let definition
  const actions = [], attached = [], detached = []
  vm.runInNewContext(source('custom-tab-bar/index.js'), {
    Component: value => { definition = value },
    getApp: () => ({ linjianTabbar: { attach: bar => attached.push(bar), detach: bar => detached.push(bar), activate: id => actions.push(id) } }),
  })
  assert.equal(definition.data.items.length, 0)
  assert.equal(definition.data.ready, false)
  const bar = {}
  definition.lifetimes.attached.call(bar)
  definition.pageLifetimes.show.call(bar)
  definition.methods.activate({ currentTarget: { dataset: { id: 'publish' } } })
  definition.lifetimes.detached.call(bar)
  assert.deepEqual(attached, [bar, bar])
  assert.deepEqual(detached, [bar])
  assert.deepEqual(actions, ['publish'])
})
