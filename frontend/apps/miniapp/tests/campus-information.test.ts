import { readFileSync } from 'node:fs'
import { defineComponent, h, nextTick, ref } from 'vue'
import { expect, it, vi } from 'vitest'
import { component, findAll, mount, tap, text } from './support/vue-component'

function setup(kind?: 'legal' | 'tag', empty = false) {
  const blank = defineComponent({ setup: () => () => h('view') })
  const list = vi.fn().mockResolvedValue(empty ? [] : [
    { id: 'service', kind: 'service', title: '报修服务', content: '服务正文', action: { kind: 'none' } },
    { id: 'legal', kind: 'legal', title: '校园须知', content: '说明正文', action: { kind: 'none' } },
    { id: 'banner', kind: 'banner', title: '广告内容', content: '', action: { kind: 'none' } },
  ])
  const tags = vi.fn().mockResolvedValue({ tags: [{ id: 'tag', name: '学习交流' }] })
  const status = defineComponent({ props: ['empty'], setup: props => () => h('view', { 'data-empty': props.empty }) })
  const screen = component(new URL('../src/features/resources/Resources.vue', import.meta.url), {
    './api': { resourcesApi: { list }, homeMenuApi: { public: tags } },
    './actions': { executeResourceAction: vi.fn(), actionLabel: () => '' },
    '../school-context/context': { requireSchool: async () => true, schoolContext: { id: 'school' }, scopeKey: () => 'scope' },
    '../../shared/use-page': { usePage: () => ({ busy: ref(false), error: ref(''), run: async (action: () => Promise<void>) => action() }) },
    '../../shared/PageStatus.vue': { default: status },
    '../media/PrivateImage.vue': { default: blank },
  })
  return { mounted: mount(screen, { kind }), list, tags }
}

it('defaults to services and switches categories without mixing content or reloading', async () => {
  const { mounted, list, tags } = setup()
  await mounted.exposed.value.show()
  await nextTick()
  expect(text(mounted.root)).toContain('校园信息')
  expect(text(mounted.root)).toContain('报修服务')
  expect(text(mounted.root)).not.toContain('校园须知')
  expect(text(mounted.root)).not.toContain('广告内容')
  const buttons = findAll(mounted.root, target => target.type === 'button')
  expect(buttons[0].props.class).toContain('selected')
  tap(buttons[1])
  await nextTick()
  expect(text(mounted.root)).toContain('校园须知')
  expect(text(mounted.root)).not.toContain('报修服务')
  expect(buttons[1].props.class).toContain('selected')
  await mounted.exposed.value.show()
  await nextTick()
  expect(text(mounted.root)).toContain('校园须知')
  tap(buttons[0])
  await nextTick()
  expect(text(mounted.root)).toContain('报修服务')
  expect(text(mounted.root)).not.toContain('校园须知')
  expect(list).toHaveBeenCalledTimes(2)
  expect(list).toHaveBeenCalledWith('school')
  expect(tags).not.toHaveBeenCalled()
  mounted.unmount()
})

it('opens legacy school information links on the information tab', async () => {
  const { mounted } = setup('legal')
  await mounted.exposed.value.show()
  await nextTick()
  expect(text(mounted.root)).toContain('校园信息')
  expect(text(mounted.root)).toContain('校园须知')
  expect(text(mounted.root)).not.toContain('报修服务')
  tap(findAll(mounted.root, target => target.type === 'button')[0])
  await nextTick()
  expect(text(mounted.root)).toContain('报修服务')
  mounted.unmount()
})

it('shows an empty state for either empty category', async () => {
  const { mounted } = setup(undefined, true)
  await mounted.exposed.value.show()
  await nextTick()
  expect(findAll(mounted.root, target => target.props['data-empty'] === true)).toHaveLength(1)
  tap(findAll(mounted.root, target => target.type === 'button')[1])
  await nextTick()
  expect(findAll(mounted.root, target => target.props['data-empty'] === true)).toHaveLength(1)
  mounted.unmount()
})

it('preserves the separate campus tags page', async () => {
  const { mounted, list, tags } = setup('tag')
  await mounted.exposed.value.show()
  await nextTick()
  expect(text(mounted.root)).toContain('校园标签')
  expect(text(mounted.root)).toContain('学习交流')
  expect(findAll(mounted.root, target => target.type === 'button')).toHaveLength(0)
  expect(list).not.toHaveBeenCalled()
  expect(tags).toHaveBeenCalledWith('school')
  mounted.unmount()
})

it('exposes only one campus information entry on the profile', () => {
  const profile = readFileSync(new URL('../src/features/community/Profile.vue', import.meta.url), 'utf8')
  expect(profile.match(/title="校园信息"/g)).toHaveLength(1)
  expect(profile).not.toMatch(/title="(?:校园服务|学校说明)"/)
  const pages = JSON.parse(readFileSync(new URL('../src/pages.json', import.meta.url), 'utf8')).pages
  for (const path of ['pages/index/category', 'pages/user/legal']) {
    expect(pages.find((page: { path: string }) => page.path === path).style.navigationBarTitleText).toBe('校园信息')
  }
})
