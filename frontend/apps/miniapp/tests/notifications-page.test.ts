import { afterEach, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
import * as notificationList from '../src/features/community/notification-list'
import { component, findAll, mount, tap, text } from './support/vue-component'

afterEach(() => vi.unstubAllGlobals())
async function settle() {
  await Promise.resolve()
  await Promise.resolve()
  await Promise.resolve()
  await nextTick()
}
function setup() {
  const communityApi = {
    notifications: vi.fn().mockResolvedValue({ items: [{ id: 'notice', kind: 'comment', created_at: '2026-11-24T02:30:00Z', read_at: null }], page: { has_more: false } }),
    read: vi.fn().mockResolvedValue(null),
  }
  const summary = { state: { error: '' }, refresh: vi.fn().mockResolvedValue(undefined), invalidate: vi.fn().mockResolvedValue(undefined) }
  const requireSchool = vi.fn().mockResolvedValue(true)
  const stop = vi.fn(), pageScrollTo = vi.fn()
  vi.stubGlobal('uni', { pageScrollTo })
  const screen = component(new URL('../src/features/community/Notifications.vue', import.meta.url), {
    './api': { communityApi },
    '../school-context/context': {
      schoolContext: { id: 'school', schools: [{ id: 'school', theme_primary: '#FFFFFF', theme_text_color: '#FFFFFF' }] },
      requireSchool, scopeKey: () => 'scope', onScopeChange: () => stop,
    },
    '../identity/session': { session: { user: { id: 'account' } } },
    '../school-context/branding': {
      defaultTheme: { theme_primary: '#6A5AF9', theme_text_color: '#FFFFFF' },
      themeColor: (value: string, fallback: string) => value || fallback,
    },
    '../../shared/use-page': { handlePageError: (error: Error) => error.message },
    './notification-state': { notificationSummary: summary },
    './notification-list': notificationList,
  })
  return { mounted: mount(screen), communityApi, summary, requireSchool, stop, pageScrollTo }
}

it('renders five usable tabs, switches to the correct empty state and retains cached all messages', async () => {
  const { mounted, communityApi, pageScrollTo, stop } = setup()
  await mounted.exposed.value.show()
  await settle()
  expect(findAll(mounted.root, target => target.type === 'button' && String(target.props.id).startsWith('notification-tab-'))).toHaveLength(5)
  expect(text(mounted.root)).toContain('收到评论')
  expect(text(mounted.root)).toContain('未读')
  expect(findAll(mounted.root, target => target.props['aria-label'] === '全部，已选中')).toHaveLength(1)
  expect(findAll(mounted.root, target => String(target.props.style).includes('--notification-tab-text:#000000'))).toHaveLength(1)
  communityApi.notifications.mockResolvedValueOnce({ items: [], page: { has_more: false } })
  tap(findAll(mounted.root, target => target.props.id === 'notification-tab-moderation')[0])
  await settle()
  expect(communityApi.notifications).toHaveBeenLastCalledWith('school', 1, { kind: 'moderation' })
  expect(text(mounted.root)).toContain('暂无审核结果通知')
  expect(text(mounted.root)).not.toContain('收到评论')
  expect(pageScrollTo).toHaveBeenCalledWith({ scrollTop: 0, duration: 0 })
  tap(findAll(mounted.root, target => target.props.id === 'notification-tab-all')[0])
  await settle()
  expect(text(mounted.root)).toContain('收到评论')
  expect(communityApi.notifications).toHaveBeenCalledTimes(2)
  expect(communityApi.read).not.toHaveBeenCalled()
  mounted.unmount()
  expect(stop).toHaveBeenCalledOnce()
})

it('keeps confirmed reads visible when the shared unread summary fails', async () => {
  const { mounted, communityApi, summary } = setup()
  await mounted.exposed.value.show()
  summary.invalidate.mockImplementationOnce(async () => { summary.state.error = 'offline' })
  tap(findAll(mounted.root, target => target.type === 'button' && text(target) === '标记已读')[0])
  await settle()
  expect(communityApi.read).toHaveBeenCalledExactlyOnceWith('school', 'notice')
  expect(text(mounted.root)).toContain('未读数量暂未更新')
  expect(text(mounted.root)).not.toContain('标记已读失败')
  expect(findAll(mounted.root, target => target.type === 'button' && text(target) === '已读')[0].props.disabled).toBe(true)
  mounted.unmount()
})

it('retries failed school preparation and does not start requests after hiding during preparation', async () => {
  const { mounted, requireSchool, communityApi, summary } = setup()
  requireSchool.mockRejectedValueOnce(new Error('学校加载失败'))
  await mounted.exposed.value.show()
  await settle()
  expect(text(mounted.root)).toContain('学校加载失败')
  expect(communityApi.notifications).not.toHaveBeenCalled()
  tap(findAll(mounted.root, target => target.type === 'button' && text(target) === '重试')[0])
  await settle()
  expect(communityApi.notifications).toHaveBeenCalledOnce()
  expect(summary.refresh).toHaveBeenLastCalledWith(true)
  let finish!: (value: boolean) => void
  requireSchool.mockReturnValueOnce(new Promise<boolean>(resolve => { finish = resolve }))
  const show = mounted.exposed.value.show()
  mounted.exposed.value.hide()
  finish(true)
  await show
  expect(communityApi.notifications).toHaveBeenCalledOnce()
  requireSchool.mockRejectedValueOnce(new Error('返回时学校加载失败'))
  await mounted.exposed.value.show()
  await settle()
  expect(text(mounted.root)).toContain('返回时学校加载失败')
  tap(findAll(mounted.root, target => target.type === 'button' && text(target) === '重试')[0])
  await settle()
  expect(communityApi.notifications).toHaveBeenCalledTimes(2)
  mounted.unmount()
})
