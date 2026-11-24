import { expect, it, vi } from 'vitest'
import { defineComponent, h, nextTick } from 'vue'
import type { CommunityPost } from '../src/features/community/api'
import * as copy from '../src/features/community/post-feed-card'
import { component, findAll, mount, tap, text } from './support/vue-component'

const post: CommunityPost = {
  id: 'post',
  school_id: 'school',
  author_id: 'author',
  author_name: '作者',
  title: '帖子',
  display_label: '帖子',
  content: '正文',
  images: [],
  tags: [],
  audit_status: 'approved',
  visibility: 'visible',
  revision: 1,
  display_published_at: '2026-11-21T00:00:00Z',
  published_at_text: '2026-11-21 08:00',
  like_count: 2,
  comment_count: 1,
}
const preview = vi.fn()
const icon = defineComponent({ setup: () => () => h('text') })
const image = defineComponent({
  props: ['school', 'source', 'variant'],
  setup: (props) => () =>
    h('image', {
      ...props,
      onTap: (event: { stopPropagation: () => void }) => {
        event.stopPropagation()
        preview()
      },
    }),
})
const card = component(new URL('../src/features/community/PostFeedCard.vue', import.meta.url), {
  './post-feed-card': copy,
  '../../shared/AppIcon.vue': { default: icon },
  '../media/PrivateImage.vue': { default: image },
})

it.each([
  ['', '只有正文', { title: '', summary: '只有正文' }],
  ['只有标题', '', { title: '', summary: '只有标题' }],
  [' 同一标题 ', '同一\n标题', { title: '', summary: '同一\n标题' }],
  ['不同标题', '有独立正文', { title: '不同标题', summary: '有独立正文' }],
  ['', '', { title: '', summary: '' }],
])('deduplicates presentation without modifying source content: %s', (title, content, expected) => {
  const input = { title, content }
  expect(copy.postFeedCopy(input)).toEqual(expected)
  expect(input).toEqual({ title, content })
})

it('renders a single card, uses the whole card for navigation and only the first image for preview', async () => {
  const open = vi.fn()
  const mounted = mount(card, { post, school: 'school', onOpen: open })
  expect(findAll(mounted.root, (target) => target.type === 'image')).toHaveLength(0)
  expect(text(mounted.root)).toContain('作者')
  expect(text(mounted.root)).not.toContain('查看全文')
  expect(text(mounted.root)).not.toContain('发布时间：')
  const link = findAll(mounted.root, (target) => target.props.role === 'link')[0]
  tap(link)
  expect(open).toHaveBeenCalledWith(post)
  mounted.props.post = { ...post, images: ['portrait', 'second'] }
  await nextTick()
  const images = findAll(mounted.root, (target) => target.type === 'image')
  expect(images).toHaveLength(1)
  expect(images[0].props).toMatchObject({ school: 'school', source: 'portrait', variant: 'feed' })
  tap(images[0])
  expect(preview).toHaveBeenCalledOnce()
  expect(open).toHaveBeenCalledOnce()
  mounted.unmount()
})

it('keeps moderation and visibility information only on personal posts and reacts to refreshed props', async () => {
  const mounted = mount(card, { post: { ...post, title: '正文' }, school: 'school', mine: true })
  expect(findAll(mounted.root, (target) => target.props.class === 'feed-title')).toHaveLength(0)
  expect(text(mounted.root)).toContain('状态：正常 · 已通过审核')
  for (const [audit_status, label] of [
    ['pending', '待审核'],
    ['rejected', '未通过审核'],
  ] as const) {
    mounted.props.post = { ...post, audit_status, visibility: 'hidden' }
    await nextTick()
    expect(text(mounted.root)).toContain(`状态：隐藏 · ${label}`)
  }
  mounted.props.post = {
    ...post,
    author_name: '',
    content: 'long'.repeat(1000),
    visibility: 'offline',
  }
  await nextTick()
  expect(text(mounted.root)).toContain('校园成员')
  expect(text(mounted.root)).toContain('系统下架')
  expect(text(mounted.root)).toContain(`${'long'.repeat(60)}…`)
  expect(text(mounted.root)).not.toContain('long'.repeat(1000))
  mounted.props.mine = false
  await nextTick()
  expect(text(mounted.root)).not.toContain('状态：')
  mounted.unmount()
})
