import { expect, it, vi } from 'vitest'
import { createNotificationSummary, notificationBadge } from '../src/features/community/notification-summary'

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (error: Error) => void
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no })
  return { promise, resolve, reject }
}
const summary = (unread_count: number) => ({ id: 'school', school_id: 'school', unread_count })
function setup() {
  let scope: { key: string; school: string } | undefined = { key: 'account:school:1', school: 'school' }
  let now = 100_000
  const read = vi.fn().mockResolvedValue(summary(4)), failure = vi.fn()
  const store = createNotificationSummary({ scope: () => scope, read, failure, now: () => now })
  return { store, read, failure, scope: (value: typeof scope) => { scope = value }, advance: () => { now += 31_000 } }
}
it('formats only valid positive integers and caps presentation at 99+', () => {
  expect([undefined, 0, -1, 0.5, NaN, Number.MAX_SAFE_INTEGER + 1, 1, 99, 100].map(notificationBadge)).toEqual(['', '', '', '', '', '', '1', '99', '99+'])
})
it('coalesces root page reads and revalidates only when stale', async () => {
  const { store, read, advance } = setup()
  const first = store.refresh()
  expect(store.refresh()).toBe(first)
  await first
  await store.refresh()
  expect(read).toHaveBeenCalledOnce()
  expect(store.state.count).toBe(4)
  advance()
  await store.refresh()
  expect(read).toHaveBeenCalledTimes(2)
})
it('discards a pre-read snapshot and queues a fresh read after a mutation', async () => {
  const { store, read } = setup()
  const old = deferred<ReturnType<typeof summary>>(), fresh = deferred<ReturnType<typeof summary>>()
  read.mockReturnValueOnce(old.promise).mockReturnValueOnce(fresh.promise)
  const operation = store.refresh()
  const changed = store.invalidate()
  old.resolve(summary(9))
  await Promise.resolve(); await Promise.resolve()
  expect(store.state.count).toBeUndefined()
  expect(read).toHaveBeenCalledTimes(2)
  fresh.resolve(summary(8))
  await Promise.all([operation, changed])
  expect(store.state.count).toBe(8)
})
it('clears on account/school loss and ignores late responses without relying on cancellation', async () => {
  const { store, read, scope } = setup()
  const old = deferred<ReturnType<typeof summary>>()
  read.mockReturnValueOnce(old.promise)
  const operation = store.refresh()
  scope({ key: 'different:school:2', school: 'new-school' })
  store.sync()
  await store.refresh()
  old.resolve(summary(99))
  await operation
  expect(store.state.count).toBe(4)
  scope(undefined)
  store.sync()
  expect(store.state.count).toBeUndefined()
  await store.refresh()
  expect(read).toHaveBeenCalledTimes(2)
})
it('keeps first failures unknown and previous values intact while allowing retry', async () => {
  const { store, read, failure } = setup()
  read.mockRejectedValueOnce(new Error('offline'))
  await store.refresh()
  expect(store.state.count).toBeUndefined()
  expect(store.state.error).toBe('offline')
  await store.refresh()
  expect(store.state.error).toBe('')
  read.mockRejectedValueOnce(new Error('offline again'))
  await store.invalidate()
  expect(store.state.count).toBe(4)
  expect(store.state.error).toBe('offline again')
  read.mockResolvedValue(summary(0))
  await store.refresh()
  expect(store.state.count).toBe(0)
  expect(store.state.error).toBe('')
  expect(failure).toHaveBeenCalledTimes(2)
})
