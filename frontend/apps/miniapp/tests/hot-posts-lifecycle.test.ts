import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { createHotPosts } from '../src/features/community/use-hot-posts'
import type { HotPostsResult } from '../src/features/community/api'

function result(school: string): HotPostsResult {
  return { items: [{ id: school, school_id: school, rank: 1, title: school, like_count: 3, comment_count: 0 }], meta: { window_start: '', window_end: '', ranked_at: '', validated_at: '', limit: 10 } }
}
function deferred() {
  let resolve!: (value: HotPostsResult) => void
  const promise = new Promise<HotPostsResult>(done => { resolve = done })
  return { promise, resolve }
}
function setup() {
  let scope: { key: string; school: string } | undefined = { key: 'a:user', school: 'a' }
  const read = vi.fn().mockImplementation(async (school: string) => result(school))
  const env = { scope: () => scope, read, navigate: vi.fn(), failure: vi.fn() }
  return { board: createHotPosts(env), read, env, scope: (next: typeof scope) => { scope = next } }
}
beforeEach(() => vi.useFakeTimers())
afterEach(() => { vi.clearAllTimers(); vi.useRealTimers() })

it('aborts hidden reads, ignores late completions and revalidates before showing again', async () => {
  const { board, read } = setup()
  const first = deferred()
  read.mockReturnValueOnce(first.promise)
  const pending = board.show()
  await Promise.resolve()
  const signal = read.mock.calls[0][2]
  const abort = vi.fn()
  signal.addEventListener('abort', abort)
  board.hide()
  expect(signal.aborted).toBe(true)
  expect(abort).toHaveBeenCalledOnce()
  first.resolve(result('a'))
  await pending
  expect(board.items.value).toEqual([])
  await vi.advanceTimersByTimeAsync(120_000)
  expect(read).toHaveBeenCalledOnce()
  await board.show()
  expect(read).toHaveBeenCalledTimes(2)
  expect(board.items.value[0].school_id).toBe('a')
  board.dispose()
})
it('never lets a late school/account/scope response replace a fresh result', async () => {
  const { board, read, scope, env } = setup()
  const first = deferred()
  read.mockReturnValueOnce(first.promise)
  const pending = board.show()
  await Promise.resolve()
  scope({ key: 'b:user', school: 'b' })
  await board.show()
  first.resolve(result('a'))
  await pending
  expect(board.items.value[0].school_id).toBe('b')
  scope({ key: 'b:another-user', school: 'b' })
  board.open(board.items.value[0])
  expect(env.navigate).not.toHaveBeenCalled()
  await board.show()
  expect(read).toHaveBeenCalledTimes(3)
  scope(undefined)
  await board.show()
  expect(board.items.value).toEqual([])
  board.dispose()
  scope({ key: 'c:user', school: 'c' })
  await board.show()
  expect(read).toHaveBeenCalledTimes(3)
})
it('queues only one follow-up refresh while a request is running and never overlaps polling', async () => {
  const { board, read } = setup()
  const first = deferred()
  read.mockReturnValueOnce(first.promise)
  const initial = board.show()
  await Promise.resolve()
  const manual = board.refresh()
  const another = board.refresh()
  board.invalidate({ scope: 'a:user', school: 'a' })
  await vi.advanceTimersByTimeAsync(120_000)
  expect(read).toHaveBeenCalledOnce()
  first.resolve(result('a'))
  await Promise.all([initial, manual, another])
  expect(read).toHaveBeenCalledTimes(2)
  expect(board.loading.value).toBe(false)
  board.dispose()
})
it('cleans up timers on disposal and ignores stale security errors after a scope switch', async () => {
  const { board, read, scope, env } = setup()
  let reject!: (reason: unknown) => void
  read.mockReturnValueOnce(new Promise<HotPostsResult>((_resolve, fail) => { reject = fail }))
  const pending = board.show()
  await Promise.resolve()
  scope({ key: 'b:user', school: 'b' })
  await board.show()
  reject({ status: 403 })
  await pending
  expect(env.failure).not.toHaveBeenCalled()
  board.invalidate({ scope: 'b:user', school: 'b' })
  board.dispose()
  expect(vi.getTimerCount()).toBe(0)
  await vi.advanceTimersByTimeAsync(120_000)
  expect(read).toHaveBeenCalledTimes(2)
})
it('suspends on security failures before scope invalidation can trigger an immediate request loop', async () => {
  const { board, read, scope, env } = setup()
  read.mockRejectedValueOnce({ status: 403 })
  env.failure.mockImplementation(() => {
    scope({ key: 'a:next-generation:user', school: 'a' })
    void board.show()
  })
  await board.show()
  await vi.advanceTimersByTimeAsync(120_000)
  expect(read).toHaveBeenCalledOnce()
  expect(board.visible.value).toBe(false)
  expect(board.items.value).toEqual([])
  expect(vi.getTimerCount()).toBe(0)
  await board.show(true)
  expect(read).toHaveBeenCalledTimes(2)
  expect(board.items.value).toHaveLength(1)
  board.dispose()
})
