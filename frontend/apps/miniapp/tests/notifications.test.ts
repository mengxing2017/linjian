import { expect, it, vi } from 'vitest'
import { createNotificationList, formatNotificationTime, notificationForeground, notificationTabs } from '../src/features/community/notification-list'
import type { Notification } from '../src/features/community/api'

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (error: Error) => void
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no })
  return { promise, resolve, reject }
}
const item = (id = 'one', kind = 'comment'): Notification => ({ id, kind, object_id: 'post', created_at: '2026-11-24T02:30:00Z', read_at: null })
const result = (items = [item()], has_more = false) => ({ items, page: { has_more } })
function setup() {
  let scope: { key: string; school: string } | undefined = { key: 'account:school', school: 'school' }
  const options = {
    scope: () => scope,
    read: vi.fn().mockResolvedValue(result()),
    markRead: vi.fn().mockResolvedValue(null),
    refreshSummary: vi.fn().mockResolvedValue(undefined),
    invalidateSummary: vi.fn().mockResolvedValue(undefined),
    failure: vi.fn((error: unknown) => error instanceof Error ? error.message : '失败'),
    now: () => new Date('2026-11-24T03:00:00Z'),
  }
  return { board: createNotificationList(options), options, changeScope: (next: typeof scope) => { scope = next } }
}

it('loads only all on entry, uses explicit kinds and caches completed categories', async () => {
  const { board, options } = setup()
  expect(notificationTabs.map(tab => tab.label)).toEqual(['全部', '评论', '点赞', '关注', '系统'])
  await board.show()
  expect(options.read).toHaveBeenCalledExactlyOnceWith('school', 1, {})
  for (const kind of ['comment', 'like', 'follow', 'moderation'] as const) {
    await board.select(kind)
    expect(options.read).toHaveBeenLastCalledWith('school', 1, { kind })
  }
  await board.select('all')
  await board.select('all')
  expect(options.read).toHaveBeenCalledTimes(5)
  expect(options.markRead).not.toHaveBeenCalled()
})

it('does not let busy or out-of-order responses swallow the final selection', async () => {
  const { board, options } = setup()
  const old = deferred<ReturnType<typeof result>>(), comment = deferred<ReturnType<typeof result>>(), last = deferred<ReturnType<typeof result>>()
  options.read.mockReturnValueOnce(old.promise).mockReturnValueOnce(comment.promise).mockReturnValueOnce(last.promise)
  const first = board.show(), second = board.select('comment'), third = board.select('all')
  comment.resolve(result([item('comment')]))
  old.reject(new Error('old forbidden'))
  await Promise.all([first, second])
  expect(board.current.value.loading).toBe(true)
  expect(options.failure).not.toHaveBeenCalled()
  last.resolve(result([item('latest')]))
  await third
  expect(board.state.activeTab).toBe('all')
  expect(board.current.value.items.map(row => row.id)).toEqual(['latest'])
})

it('deduplicates pagination, retries the failed page and stops at the end', async () => {
  const { board, options } = setup()
  options.read.mockResolvedValueOnce(result([item('one')], true))
  await board.show()
  const pending = deferred<ReturnType<typeof result>>()
  options.read.mockReturnValueOnce(pending.promise)
  const append = board.loadMore()
  await board.loadMore()
  expect(options.read).toHaveBeenCalledTimes(2)
  pending.reject(new Error('offline'))
  await append
  expect(board.current.value.page).toBe(1)
  expect(board.current.value.items).toHaveLength(1)
  expect(board.current.value.appendError).toBe('offline')
  options.read.mockResolvedValueOnce(result([item('one'), item('two')]))
  await board.loadMore()
  expect(options.read).toHaveBeenLastCalledWith('school', 2, {})
  expect(board.current.value.items.map(row => row.id)).toEqual(['one', 'two'])
  await board.loadMore()
  expect(options.read).toHaveBeenCalledTimes(3)
})

it('refresh invalidates old appends and lazy-invalidates other categories', async () => {
  const { board, options } = setup()
  await board.show()
  await board.select('comment')
  options.read.mockResolvedValueOnce(result([item('old')], true))
  await board.select('like')
  const pending = deferred<ReturnType<typeof result>>()
  options.read.mockReturnValueOnce(pending.promise)
  const append = board.loadMore()
  options.read.mockResolvedValueOnce(result([item('fresh')]))
  await board.refresh()
  pending.resolve(result([item('stale')]))
  await append
  expect(board.current.value.items.map(row => row.id)).toEqual(['fresh'])
  expect(board.current.value.page).toBe(1)
  expect(options.refreshSummary).toHaveBeenLastCalledWith(true)
  const before = options.read.mock.calls.length
  await board.select('comment')
  expect(options.read).toHaveBeenCalledTimes(before + 1)
})

it('retains same-category data on failed refresh but prevents stale pagination', async () => {
  const { board, options } = setup()
  options.read.mockResolvedValueOnce(result([item()], true))
  await board.show()
  options.read.mockRejectedValueOnce(new Error('refresh failed'))
  await board.refresh()
  expect(board.current.value.items).toHaveLength(1)
  expect(board.current.value.error).toBe('refresh failed')
  await board.loadMore()
  expect(options.read).toHaveBeenCalledTimes(2)
  await board.retry()
  expect(board.current.value.error).toBe('')
})

it('confirms reads across caches and never restores unread from an older list response', async () => {
  const { board, options } = setup()
  await board.show()
  await board.select('comment')
  const pending = deferred<ReturnType<typeof result>>()
  options.read.mockReturnValueOnce(pending.promise)
  const refresh = board.refresh()
  const write = deferred<null>()
  options.markRead.mockReturnValueOnce(write.promise)
  const read = board.read(item())
  await board.read(item())
  expect(options.markRead).toHaveBeenCalledOnce()
  expect(board.current.value.items[0].read_at).toBeNull()
  write.resolve(null)
  await read
  pending.resolve(result())
  await refresh
  expect(board.current.value.items[0].read_at).toBe('2026-11-24T03:00:00.000Z')
  expect(board.state.categories.all.items[0].read_at).toBeTruthy()
  expect(options.invalidateSummary).toHaveBeenCalledOnce()
  await board.read(item())
  expect(options.markRead).toHaveBeenCalledOnce()
})

it('distinguishes a failed write from a failed summary without undoing a confirmed read', async () => {
  const { board, options } = setup()
  await board.show()
  options.markRead.mockRejectedValueOnce(new Error('write failed'))
  await board.read(item())
  expect(board.state.readError).toBe('write failed')
  expect(board.current.value.items[0].read_at).toBeNull()
  expect(options.invalidateSummary).not.toHaveBeenCalled()
  options.invalidateSummary.mockRejectedValueOnce(new Error('summary failed'))
  await board.read(item())
  expect(board.state.readError).toBe('')
  expect(board.state.summaryError).toBe('summary failed')
  expect(board.current.value.items[0].read_at).toBeTruthy()
})

it('invalidates hidden responses and revalidates on return regardless of unread count', async () => {
  const { board, options } = setup()
  const pending = deferred<ReturnType<typeof result>>()
  options.read.mockReturnValueOnce(pending.promise)
  const show = board.show()
  board.hide()
  pending.reject(new Error('hidden error'))
  await show
  expect(options.failure).not.toHaveBeenCalled()
  expect(board.current.value.loading).toBe(false)
  await board.show()
  expect(options.read).toHaveBeenCalledTimes(2)
  await board.refresh()
  expect(options.read).toHaveBeenCalledTimes(3)
})

it('clears scope state and ignores old writes, reads and errors after account changes or disposal', async () => {
  const { board, options, changeScope } = setup()
  await board.show()
  const write = deferred<null>(), query = deferred<ReturnType<typeof result>>()
  options.markRead.mockReturnValueOnce(write.promise)
  options.read.mockReturnValueOnce(query.promise)
  const read = board.read(item()), select = board.select('comment')
  changeScope({ key: 'another-account:school', school: 'school' })
  board.reset()
  write.resolve(null)
  query.reject(new Error('old forbidden'))
  await Promise.all([read, select])
  expect(board.state.activeTab).toBe('all')
  expect(board.current.value.items).toEqual([])
  expect(options.invalidateSummary).not.toHaveBeenCalled()
  expect(options.failure).not.toHaveBeenCalled()
  board.dispose()
  await board.show()
  await board.select('like')
  await board.refresh()
  expect(options.read).toHaveBeenCalledTimes(2)
})

it('does not submit stale error text when error handling invalidates the scope', async () => {
  const { board, options, changeScope } = setup()
  options.failure.mockImplementation(() => { changeScope(undefined); board.reset(); return 'forbidden' })
  options.read.mockRejectedValueOnce(new Error('forbidden'))
  await board.show()
  expect(board.current.value.error).toBe('')
  expect(board.current.value.items).toEqual([])
})

it('keeps first-page errors separate from empty results and retries the active category', async () => {
  const { board, options } = setup()
  await board.show()
  options.read.mockRejectedValueOnce(new Error('category unavailable'))
  await board.select('comment')
  expect(board.current.value.loaded).toBe(false)
  expect(board.current.value.items).toEqual([])
  expect(board.current.value.error).toBe('category unavailable')
  expect(board.state.categories.all.items).toHaveLength(1)
  options.read.mockResolvedValueOnce(result([]))
  await board.retry()
  expect(options.read).toHaveBeenLastCalledWith('school', 1, { kind: 'comment' })
  expect(board.current.value.loaded).toBe(true)
  expect(board.current.value.error).toBe('')
})

it('drops pending summary errors after scope loss and does not load without a school', async () => {
  const { board, options, changeScope } = setup()
  const summary = deferred<void>()
  options.refreshSummary.mockReturnValueOnce(summary.promise)
  const show = board.show()
  changeScope(undefined)
  board.reset()
  summary.reject(new Error('old summary failure'))
  await show
  expect(options.failure).not.toHaveBeenCalled()
  expect(board.state.summaryError).toBe('')
  await board.show()
  await board.refresh()
  expect(options.read).toHaveBeenCalledOnce()
})

it('keeps empty categories distinct and formats local dates and readable theme foregrounds', async () => {
  const { board, options } = setup()
  options.read.mockResolvedValue(result([]))
  await board.show()
  await board.select('moderation')
  expect(board.current.value.loaded).toBe(true)
  expect(board.current.value.error).toBe('')
  expect(notificationTabs.find(tab => tab.id === board.state.activeTab)?.empty).toBe('暂无审核结果通知')
  const now = new Date(2026, 10, 24, 12)
  expect(formatNotificationTime(new Date(2026, 10, 24, 9, 5).toISOString(), now)).toBe('09:05')
  expect(formatNotificationTime(new Date(2026, 10, 23, 9, 5).toISOString(), now)).toBe('11月23日 09:05')
  expect(formatNotificationTime(new Date(2025, 10, 23, 9, 5).toISOString(), now)).toBe('2025年11月23日 09:05')
  expect(formatNotificationTime('invalid', now)).toBe('时间未知')
  expect(notificationForeground('#FFFFFF', '#FFFFFF')).toBe('#000000')
  expect(notificationForeground('#000000', '#000000')).toBe('#FFFFFF')
  expect(notificationForeground('#FFFFFF', '#222222')).toBe('#222222')
})
