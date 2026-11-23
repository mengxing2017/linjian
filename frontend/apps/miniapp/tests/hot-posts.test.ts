import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { createHotPosts, isHotPostsHome } from '../src/features/community/use-hot-posts'
import type { HotPostsResult } from '../src/features/community/api'

function result(school = 'school', amount = 10): HotPostsResult {
  return {
    items: Array.from({ length: amount }, (_, index) => ({ id: `post-${index}`, school_id: school, rank: index + 1, display_label: `标题${index}`, like_count: 3, comment_count: 0 })),
    meta: { window_start: '2026-11-14T04:00:00Z', window_end: '2026-11-21T04:00:00Z', ranked_at: '2026-11-21T04:00:00Z', validated_at: '2026-11-21T04:00:00Z', limit: 10 },
  }
}
function setup(reducedMotion = false) {
  let scope: { key: string; school: string } | undefined = { key: 'scope:user', school: 'school' }
  const env = { scope: () => scope, read: vi.fn().mockResolvedValue(result()), navigate: vi.fn(), failure: vi.fn(), reducedMotion }
  const board = createHotPosts(env)
  return { board, env, changeScope: (next: typeof scope) => { scope = next } }
}
beforeEach(() => vi.useFakeTimers())
afterEach(() => { vi.clearAllTimers(); vi.useRealTimers() })

it('groups only the default home discovery into up to three rows per screen', async () => {
  expect(isHotPostsHome(true, false, 'feed', '', '')).toBe(true)
  for (const [home, searchPage, mode, search, tag] of [
    [false, false, 'feed', '', ''], [true, true, 'feed', '', ''], [true, false, 'following', '', ''],
    [true, false, 'collect', '', ''], [true, false, 'mine', '', ''], [true, false, 'feed', '关键词', ''], [true, false, 'feed', '', '标签'],
  ] as const) expect(isHotPostsHome(home, searchPage, mode, search, tag)).toBe(false)
  const { board } = setup()
  await board.show()
  expect(board.pages.value.map(group => group.length)).toEqual([3, 3, 3, 1])
  expect(board.autoplay.value).toBe(true)
  board.userPaused.value = true
  expect(board.autoplay.value).toBe(false)
  board.userPaused.value = false
  board.touching.value = true
  expect(board.autoplay.value).toBe(false)
  board.touching.value = false
  expect(board.autoplay.value).toBe(true)
  board.dispose()
})
it('keeps a single screen static and respects reduced motion without disabling manual access', async () => {
  const { board, env } = setup(true)
  env.read.mockResolvedValueOnce(result('school', 3))
  await board.show()
  expect(board.autoplay.value).toBe(false)
  board.open(board.items.value[0])
  expect(env.navigate).toHaveBeenCalledWith('school', 'post-0')
  await board.refresh()
  expect(board.autoplay.value).toBe(false)
  board.userPaused.value = false
  expect(board.autoplay.value).toBe(true)
  board.open({ ...board.items.value[0], school_id: 'another' })
  expect(env.navigate).toHaveBeenCalledTimes(1)
  board.dispose()
})
it('refreshes every 60 seconds after completion, hides failures and recovers without a retry storm', async () => {
  const { board, env } = setup()
  await board.show()
  env.read.mockRejectedValueOnce(new Error('offline'))
  await vi.advanceTimersByTimeAsync(60_000)
  expect(env.read).toHaveBeenCalledTimes(2)
  expect(board.items.value).toEqual([])
  expect(board.autoplay.value).toBe(false)
  expect(env.failure).toHaveBeenCalledOnce()
  await vi.advanceTimersByTimeAsync(59_999)
  expect(env.read).toHaveBeenCalledTimes(2)
  await vi.advanceTimersByTimeAsync(1)
  expect(env.read).toHaveBeenCalledTimes(3)
  expect(board.items.value).toHaveLength(10)
  board.dispose()
  await vi.advanceTimersByTimeAsync(120_000)
  expect(env.read).toHaveBeenCalledTimes(3)
})
it('coalesces local invalidations for only the current scope and preserves the visible anchor on refresh', async () => {
  const { board, env } = setup()
  await board.show()
  board.page.value = 1
  board.invalidate({ scope: 'other', school: 'school' })
  board.invalidate({ scope: 'scope:user', school: 'other' })
  await vi.advanceTimersByTimeAsync(200)
  expect(env.read).toHaveBeenCalledTimes(1)
  for (let index = 0; index < 5; index++) board.invalidate({ scope: 'scope:user', school: 'school' })
  await vi.advanceTimersByTimeAsync(200)
  expect(env.read).toHaveBeenCalledTimes(2)
  expect(board.page.value).toBe(1)
  env.read.mockResolvedValueOnce(result('school', 0))
  await board.refresh()
  expect(board.page.value).toBe(0)
  board.changePage(3)
  expect(board.page.value).toBe(0)
  expect(board.items.value).toEqual([])
  board.dispose()
})
