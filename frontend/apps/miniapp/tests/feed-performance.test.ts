import { afterEach, expect, it, vi } from 'vitest'
import { defineComponent, h, nextTick, ref } from 'vue'
import * as pageWindow from '../src/shared/page-window'
import * as concurrency from '../src/shared/concurrency'
import * as feedCopy from '../src/features/community/post-feed-card'
import * as invalidations from '../src/features/community/post-invalidations'
import { component, findAll, mount, text } from './support/vue-component'

afterEach(() => vi.unstubAllGlobals())

it('limits feed models and mounted cards, and avoids a duplicate school read during initial show', async () => {
  const blank = defineComponent({ setup: (_, { expose }) => {
    expose({ show: vi.fn(), hide: vi.fn(), refresh: vi.fn() })
    return () => h('view')
  } })
  const card = defineComponent({ props: ['post'], setup: props => () => h('view', { class: 'test-card' }, props.post.content) })
  const requireSchool = vi.fn().mockResolvedValue(true)
  const posts = vi.fn(async (_school: string, query: { page: number }) => ({
    items: [{ id: `page-${query.page}`, title: '标题', content: '长正文'.repeat(1000), images: [] }],
    page: { has_more: true },
  }))
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
    './api': { communityApi: { posts }, homeMenuApi: { public: vi.fn().mockResolvedValue({ tags: [], settings: { latest_visible: true, latest_label: '最新' } }) } },
    '../school-context/context': { requireSchool, schoolContext: { id: 'school', generation: 0, schools: [{ id: 'school', name: '学校' }] }, scopeKey: () => 'scope' },
    './post-invalidations': invalidations,
    '../identity/session': { session: { user: { id: 'user', bound_school_id: 'school' } } },
    '../../shared/use-page': { navigate: vi.fn(), usePage: () => ({ busy: ref(false), error: ref(''), run: async (action: (current: () => boolean) => Promise<void>) => action(() => true) }) },
    '../../shared/PageStatus.vue': { default: blank },
    '../../shared/AppIcon.vue': { default: blank },
    './PostFeedCard.vue': { default: card },
    '../school-context/SchoolHeader.vue': { default: blank },
  })
  invalidations.clearPostInvalidations()
  const mounted = mount(screen, { homePage: true })
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
