import { afterEach, expect, it, vi } from 'vitest'
import { defineComponent, h, nextTick, reactive, ref } from 'vue'
import * as pageWindow from '../src/shared/page-window'
import * as concurrency from '../src/shared/concurrency'
import * as feedCopy from '../src/features/community/post-feed-card'
import * as invalidations from '../src/features/community/post-invalidations'
import { component, findAll, mount, text } from './support/vue-component'

afterEach(() => vi.unstubAllGlobals())

function setup(schoolContext = reactive({ id: 'school', generation: 0, school: { id: 'school', name: '学校' } as { id: string; name: string } | undefined, loaded: true }), props: Record<string, unknown> = { homePage: true }) {
  const blank = defineComponent({ setup: (_, { expose }) => {
    expose({ show: vi.fn(), hide: vi.fn(), refresh: vi.fn() })
    return () => h('view')
  } })
  const card = defineComponent({ props: ['post', 'mine'], setup: props => () => h('view', { class: 'test-card', 'data-mine': props.mine }, props.post.content) })
  const requireSchool = vi.fn().mockResolvedValue(true)
  const posts = vi.fn(async (_school: string, query: { page: number }) => ({
    items: [{ id: `page-${query.page}`, title: '标题', content: '长正文'.repeat(1000), images: [] }],
    page: { has_more: true },
  }))
  const homeMenu = vi.fn().mockResolvedValue({ tags: [], settings: { latest_visible: true, latest_label: '最新' } })
  vi.stubGlobal('uni', { pageScrollTo: vi.fn() })
  const screen = component(new URL('../src/features/community/Feed.vue', import.meta.url), {
    '../../shared/page-window': pageWindow,
    '../../shared/PageWindow.vue': { default: component(new URL('../src/shared/PageWindow.vue', import.meta.url), { './page-window': pageWindow }) },
    './post-feed-card': feedCopy,
    '../../shared/concurrency': concurrency,
    '../statistics/activity': { recordPage: vi.fn() },
    '../operations/HomeOperations.vue': { default: blank },
    '../operations/HomeShortcuts.vue': { default: blank },
    './HotPosts.vue': { default: blank },
    './use-hot-posts': { isHotPostsHome: () => true },
    './api': { communityApi: { posts }, homeMenuApi: { public: homeMenu } },
    '../school-context/context': { requireSchool, schoolContext, scopeKey: () => 'scope' },
    './post-invalidations': invalidations,
    '../identity/session': { session: { user: { id: 'user', bound_school_id: 'school' } } },
    '../../shared/use-page': { navigate: vi.fn(), usePage: () => ({ busy: ref(false), error: ref(''), run: async (action: (current: () => boolean) => Promise<void>) => action(() => true) }) },
    '../../shared/PageStatus.vue': { default: blank },
    '../../shared/AppIcon.vue': { default: blank },
    './PostFeedCard.vue': { default: card },
    '../school-context/SchoolHeader.vue': { default: blank },
  })
  invalidations.clearPostInvalidations()
  const mounted = mount(screen, props)
  return { mounted, requireSchool, posts, homeMenu }
}

it('limits feed models and mounted cards, and avoids a duplicate school read during initial show', async () => {
  const { mounted, requireSchool, posts } = setup()
  await mounted.exposed.value.show(); await nextTick()
  expect(requireSchool).toHaveBeenCalledOnce()
  const cards = () => findAll(mounted.root, target => target.props.class === 'test-card')
  expect(text(cards()[0])).toHaveLength(241)
  await mounted.exposed.value.loadMore()
  await mounted.exposed.value.loadMore(); await nextTick()
  expect(cards()).toHaveLength(3)
  await mounted.exposed.value.loadMore(); await nextTick()
  expect(cards()).toHaveLength(1)
  expect(posts).toHaveBeenLastCalledWith('school', expect.objectContaining({ page: 4 }))
  mounted.unmount()
})

it('does not show school unavailability before the initial school response or while retrying', async () => {
  const schoolContext = reactive({ id: '', generation: 0, school: undefined as { id: string; name: string } | undefined, loaded: false })
  const { mounted } = setup(schoolContext)
  const unavailable = '学校暂不可用，请联系管理员'
  expect(text(mounted.root)).not.toContain(unavailable)
  schoolContext.loaded = true
  await nextTick()
  expect(text(mounted.root)).toContain(unavailable)
  schoolContext.loaded = false
  await nextTick()
  expect(text(mounted.root)).not.toContain(unavailable)
  schoolContext.school = { id: 'school', name: '学校' }
  schoolContext.id = 'school'
  schoolContext.loaded = true
  await nextTick()
  expect(text(mounted.root)).not.toContain(unavailable)
  mounted.unmount()
})

it('keeps my posts restricted to the current user without unrelated entry points or home-menu requests', async () => {
  const { mounted, posts, homeMenu } = setup(undefined, { initial: 'mine', searchPage: true, lockedMode: true })
  homeMenu.mockRejectedValue(new Error('首页配置不可用'))
  await mounted.exposed.value.show()
  await nextTick()
  expect(text(mounted.root)).toContain('我的发布')
  for (const entry of ['发现', '关注', '收藏', '最新', '校园社区']) {
    expect(text(mounted.root)).not.toContain(entry)
  }
  expect(findAll(mounted.root, target => target.type === 'input')).toHaveLength(1)
  expect(findAll(mounted.root, target => target.type === 'scroll-view')).toHaveLength(0)
  const cards = findAll(mounted.root, target => target.props.class === 'test-card')
  expect(cards).toHaveLength(1)
  expect(cards[0].props['data-mine']).toBe(true)
  await mounted.exposed.value.loadMore()
  await mounted.exposed.value.refresh()
  expect(posts.mock.calls.map(([, query]) => query)).toEqual([
    expect.objectContaining({ page: 1, mine: true, kind: undefined, tag: '', search: '' }),
    expect.objectContaining({ page: 2, mine: true, kind: undefined, tag: '', search: '' }),
    expect.objectContaining({ page: 1, mine: true, kind: undefined, tag: '', search: '' }),
  ])
  expect(homeMenu).not.toHaveBeenCalled()
  mounted.unmount()
})

it('keeps search available on an empty my-posts page without discovery entry points', async () => {
  const { mounted, posts, homeMenu } = setup(undefined, { initial: 'mine', searchPage: true, lockedMode: true })
  posts.mockResolvedValue({ items: [], page: { has_more: false } })
  await mounted.exposed.value.show()
  await nextTick()
  expect(text(mounted.root)).toBe('我的发布搜索')
  expect(findAll(mounted.root, target => target.type === 'input')).toHaveLength(1)
  expect(findAll(mounted.root, target => target.type === 'button')).toHaveLength(1)
  expect(findAll(mounted.root, target => target.type === 'scroll-view')).toHaveLength(0)
  expect(homeMenu).not.toHaveBeenCalled()
  mounted.unmount()
})

it('searches only my posts and preserves the submitted keyword across pagination and refresh', async () => {
  const { mounted, posts, homeMenu } = setup(undefined, { initial: 'mine', searchPage: true, lockedMode: true })
  await mounted.exposed.value.show()
  await nextTick()
  const input = findAll(mounted.root, target => target.type === 'input')[0]
  expect(input.props.placeholder).toBe('搜索我的帖子')
  const write = (value: string) => (input.props.onInput as (event: unknown) => void)({ detail: { value } })
  const submit = () => (input.props.onConfirm as () => Promise<void>)()
  write('  本人帖子关键词  ')
  await submit()
  write('尚未提交的新关键词')
  await mounted.exposed.value.loadMore()
  await mounted.exposed.value.refresh()
  expect(posts.mock.calls.slice(1).map(([, query]) => query)).toEqual([
    expect.objectContaining({ page: 1, mine: true, kind: undefined, tag: '', search: '本人帖子关键词' }),
    expect.objectContaining({ page: 2, mine: true, kind: undefined, tag: '', search: '本人帖子关键词' }),
    expect.objectContaining({ page: 1, mine: true, kind: undefined, tag: '', search: '本人帖子关键词' }),
  ])
  const button = findAll(mounted.root, target => target.type === 'button' && text(target) === '搜索')[0]
  await (button.props.onTap as () => Promise<void>)()
  expect(posts).toHaveBeenLastCalledWith('school', expect.objectContaining({ page: 1, mine: true, search: '尚未提交的新关键词' }))
  write('')
  await submit()
  expect(posts).toHaveBeenLastCalledWith('school', expect.objectContaining({ page: 1, mine: true, search: '' }))
  expect(homeMenu).not.toHaveBeenCalled()
  mounted.unmount()
})
