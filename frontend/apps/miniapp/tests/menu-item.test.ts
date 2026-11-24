import { readFileSync } from 'node:fs'
import { defineComponent, h } from 'vue'
import { expect, it, vi } from 'vitest'
import { component, findAll, mount, tap, text } from './support/vue-component'

const source = new URL('../src/shared/MenuItem.vue', import.meta.url)
const menuItem = component(source, {
  './AppIcon.vue': { default: defineComponent({ setup: () => () => h('image') }) },
})

it('centers the logout label directly without changing the button width or activation', () => {
  const activate = vi.fn()
  const mounted = mount(menuItem, { title: '退出登录', arrow: false, danger: true, centered: true, onActivate: activate })
  const title = findAll(mounted.root, target => target.type === 'text')[0]
  expect(title.props.class).toContain('menu-item-title-centered')
  expect(text(title)).toBe('退出登录')
  const styles = readFileSync(source, 'utf8')
  expect(styles).toMatch(/\.menu-item\.centered\s*\{[^}]*justify-content:\s*center/)
  expect(styles).toMatch(/\.menu-item-title\.menu-item-title-centered\s*\{[^}]*flex:\s*0 1 auto;\s*text-align:\s*center/)
  const button = findAll(mounted.root, target => target.type === 'button')[0]
  tap(button)
  expect(activate).toHaveBeenCalledOnce()
  mounted.unmount()
})

it('keeps ordinary menu labels left aligned', () => {
  const mounted = mount(menuItem, { title: '个人信息', icon: 'person' })
  const title = findAll(mounted.root, target => target.type === 'text')[0]
  expect(title.props.class).not.toContain('menu-item-title-centered')
  mounted.unmount()
})
