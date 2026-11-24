import { afterEach, expect, it, vi } from 'vitest'
import { defineComponent, h, nextTick } from 'vue'
import { createHotPosts } from '../src/features/community/use-hot-posts'
import * as concurrency from '../src/shared/concurrency'
import { component, findAll, mount, tap, text } from './support/vue-component'

afterEach(() => {
  vi.unstubAllGlobals()
  vi.useRealTimers()
})
const icon = defineComponent({ setup: () => () => h('text') })
const empty = defineComponent({ setup: () => () => h('view') })
const marketing = defineComponent({
  setup: (_props, { expose }) => {
    expose({ show: vi.fn(), hide: vi.fn() })
    return () => h('view')
  },
})
async function settle() {
  await Promise.resolve()
  await Promise.resolve()
  await nextTick()
}

it('puts search in the compact header without offering school switching', () => {
  const header = component(
    new URL('../src/features/school-context/SchoolHeader.vue', import.meta.url),
    {
      '../../shared/AppIcon.vue': { default: icon },
      './BrandImage.vue': { default: empty },
      './branding': {
        defaultTheme: {
          theme_primary: '#5b8ff9',
          theme_secondary: '#48c5bd',
          theme_text_color: '#fff',
        },
        themeColor: (value: string, fallback: string) => value || fallback,
      },
      '@linjian/china-regions': { formatRegion: () => '测试地区' },
    },
  )
  const search = vi.fn()
  const school = { id: 'school', name: '非常长的学校全称'.repeat(10), revision: 1 }
  const mounted = mount(header, {
    school,
    compact: true,
    searchable: true,
    onSearch: search,
  })
  tap(findAll(mounted.root, (target) => target.props['aria-label'] === '搜索帖子')[0])
  expect(findAll(mounted.root, (target) => target.props['aria-label'] === '选择学校')).toHaveLength(0)
  expect(search).toHaveBeenCalledOnce()
  expect(text(mounted.root)).toContain(school.name)
  mounted.unmount()
  const regular = mount(header, { school })
  expect(findAll(regular.root, (target) => target.type === 'button')).toHaveLength(0)
  expect(
    findAll(regular.root, (target) => String(target.props.class).includes('school-brand-compact')),
  ).toHaveLength(0)
  regular.unmount()
})

it('keeps every hot post reachable in compact and expanded modes, including while paused', async () => {
  vi.useFakeTimers()
  let board!: ReturnType<typeof createHotPosts>
  const read = vi
    .fn()
    .mockResolvedValue({
      items: Array.from({ length: 10 }, (_, index) => ({
        id: `post-${index}`,
        school_id: 'school',
        rank: index + 1,
        display_label: `标题${index}`,
        like_count: 3,
        comment_count: 0,
      })),
      meta: {},
    })
  const hotPosts = component(new URL('../src/features/community/HotPosts.vue', import.meta.url), {
    './api': { communityApi: { hotPosts: read } },
    './use-hot-posts': {
      createHotPosts: (env: Parameters<typeof createHotPosts>[0]) => {
        board = createHotPosts(env)
        return board
      },
    },
    './post-invalidations': { onPostInvalidated: () => vi.fn() },
    '../school-context/context': { schoolContext: { id: 'school' }, scopeKey: () => 'scope' },
    '../identity/session': { session: { user: { id: 'user' } } },
    '../../shared/use-page': { handlePageError: vi.fn(), navigate: vi.fn() },
  })
  const mounted = mount(hotPosts, { active: true })
  await settle()
  expect(findAll(mounted.root, (target) => target.type === 'swiper-item')).toHaveLength(10)
  expect(findAll(mounted.root, (target) => target.type === 'swiper')[0].props.style).toEqual({
    height: '44px',
  })
  const pause = findAll(
    mounted.root,
    (target) => target.type === 'button' && text(target) === '暂停播放',
  )[0]
  tap(pause)
  await nextTick()
  expect(board.autoplay.value).toBe(false)
  const expand = findAll(
    mounted.root,
    (target) => target.type === 'button' && text(target) === '展开',
  )[0]
  tap(expand)
  await nextTick()
  expect(findAll(mounted.root, (target) => target.type === 'swiper-item')).toHaveLength(4)
  expect(findAll(mounted.root, (target) => target.type === 'swiper')[0].props.style).toEqual({
    height: '132px',
  })
  expect(board.autoplay.value).toBe(false)
  const collapse = findAll(
    mounted.root,
    (target) => target.type === 'button' && text(target) === '收起',
  )[0]
  tap(collapse)
  await nextTick()
  expect(board.rowsPerPage.value).toBe(1)
  mounted.props.active = false
  await nextTick()
  expect(board.items.value).toHaveLength(0)
  expect(board.autoplay.value).toBe(false)
  mounted.unmount()
  expect(vi.getTimerCount()).toBe(0)
})

it('reserves one of four shortcut slots for all services and records impressions only for displayed entries', async () => {
  const featured = Array.from({ length: 6 }, (_, index) => ({
    id: `item-${index}`,
    title: `服务${index}`,
    subtitle: '详细说明',
    badge: index === 0 ? '新' : null,
    image_url: `image-${index}`,
    ends_at: null,
  }))
  const catalog = vi
    .fn()
    .mockResolvedValue({ server_time: new Date().toISOString(), items: featured, featured })
  const imagePath = vi.fn().mockResolvedValue('/image')
  const metric = vi.fn().mockResolvedValue(undefined)
  const navigateTo = vi.fn(),
    disconnect = vi.fn()
  vi.stubGlobal('uni', {
    navigateTo,
    createIntersectionObserver: () => ({
      relativeToViewport() {
        return this
      },
      observe: (_selector: string, callback: (result: { intersectionRatio: number }) => void) =>
        callback({ intersectionRatio: 1 }),
      disconnect,
    }),
  })
  const shortcuts = component(
    new URL('../src/features/operations/HomeShortcuts.vue', import.meta.url),
    {
      './api': { serviceApi: { catalog } },
      '../school-context/context': {
        schoolContext: { id: 'school' },
        scopeKey: () => 'scope',
        onScopeChange: () => vi.fn(),
      },
      '../identity/session': { session: { user: { id: 'user' } } },
      '../media/files': { imagePath, releaseImages: vi.fn() },
      '../../shared/concurrency': concurrency,
      './policy': { uuid: () => 'event' },
      './service-metrics': { sendServiceEvent: metric },
      './OfficialMarketing.vue': { default: marketing },
      '../../shared/AppIcon.vue': { default: icon },
    },
  )
  const mounted = mount(shortcuts)
  await mounted.exposed.value.show()
  await nextTick()
  const buttons = findAll(mounted.root, (target) => target.type === 'button')
  expect(buttons).toHaveLength(4)
  expect(imagePath).toHaveBeenCalledTimes(3)
  expect(text(mounted.root)).not.toContain('详细说明')
  expect(text(mounted.root)).not.toContain('服务3')
  expect(text(mounted.root)).toContain('新')
  tap(buttons[3])
  expect(navigateTo).toHaveBeenCalledWith({ url: '/pages/operations/services' })
  for (const image of findAll(mounted.root, (target) => target.type === 'image')) {
    ;(image.props.onLoad as () => void)()
  }
  await settle()
  expect(metric).toHaveBeenCalledTimes(3)
  expect(metric.mock.calls.map((call) => call[1].resource_id)).toEqual([
    'item-0',
    'item-1',
    'item-2',
  ])
  mounted.unmount()
  expect(disconnect).toHaveBeenCalledTimes(3)
})
