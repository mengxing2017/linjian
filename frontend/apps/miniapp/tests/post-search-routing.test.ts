import { afterEach, expect, it, vi } from 'vitest'
import { defineComponent, h, ref } from 'vue'
import { postSearchRoute } from '../src/features/community/search-route'
import { component, findAll, mount } from './support/vue-component'

afterEach(() => vi.unstubAllGlobals())

it('keeps personal content routes and falls back to ordinary search for unknown modes', () => {
  expect(postSearchRoute('mine')).toEqual({ view: 'mine', title: '我的发布' })
  expect(postSearchRoute('collect')).toEqual({ view: 'collect', title: '我的收藏' })
  for (const view of [undefined, '', 'following', 'admin', ['mine']]) {
    expect(postSearchRoute(view)).toEqual({ view: 'search', title: '搜索帖子' })
  }
})

it.each([
  ['mine', '我的发布'],
  ['collect', '我的收藏'],
])('locks the %s route to the personal feed instead of mounting a switchable discovery feed', (view, title) => {
  vi.stubGlobal('uni', { setNavigationBarTitle: vi.fn() })
  const feed = defineComponent({
    props: { initial: String, searchPage: Boolean, lockedMode: Boolean },
    setup: props => () => h('view', { class: 'personal-feed', mode: props.initial, locked: props.lockedMode }),
  })
  const page = component(new URL('../src/pages/index/search.vue', import.meta.url), {
    '../../features/school-context/theme': { schoolTheme: '' },
    '../../app/page-lifecycle': { useProtectedScreen: () => ({ screen: ref(), allowed: ref(true) }) },
    '../../features/community/Feed.vue': { default: feed },
    '../../features/community/Search.vue': { default: defineComponent({ setup: () => () => h('input') }) },
    '../../features/community/search-route': { postSearchRoute },
    '@dcloudio/uni-app': { onLoad: (callback: (query: { view: string }) => void) => callback({ view }) },
  })
  const mounted = mount(page)
  const rendered = findAll(mounted.root, target => target.props.class === 'personal-feed')
  expect(rendered).toHaveLength(1)
  expect(rendered[0].props.mode).toBe(view)
  expect(rendered[0].props.locked).toBe(true)
  expect(findAll(mounted.root, target => target.type === 'input')).toHaveLength(0)
  expect(uni.setNavigationBarTitle).toHaveBeenCalledWith({ title })
  mounted.unmount()
})
