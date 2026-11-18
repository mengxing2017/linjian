import { afterEach, expect, it, vi } from 'vitest'
import { recordPage } from '../src/features/statistics/activity'
import { session } from '../src/features/identity/session'
import { statisticsApi } from '../src/features/statistics/api'
vi.mock('../src/features/identity/session', () => ({ session: { user: null } }))
vi.mock('../src/features/statistics/api', () => ({ statisticsApi: { activity: vi.fn() } }))
afterEach(() => {
  session.user = null
  vi.useRealTimers()
  vi.clearAllMocks()
})
it('ignores anonymous access and retries the same event at most once', async () => {
  vi.useFakeTimers()
  recordPage('feed_open', 'school')
  expect(statisticsApi.activity).not.toHaveBeenCalled()
  session.user = { id: 'user' } as NonNullable<typeof session.user>
  vi.mocked(statisticsApi.activity).mockRejectedValue(new Error('offline'))
  recordPage('post_open', 'school', 'post')
  await vi.advanceTimersByTimeAsync(1000)
  expect(statisticsApi.activity).toHaveBeenCalledTimes(2)
  expect(vi.mocked(statisticsApi.activity).mock.calls[0][0]).toEqual(
    vi.mocked(statisticsApi.activity).mock.calls[1][0],
  )
  await vi.advanceTimersByTimeAsync(5000)
  expect(statisticsApi.activity).toHaveBeenCalledTimes(2)
})
it('does not retry into a different login session', async () => {
  vi.useFakeTimers()
  session.user = { id: 'first' } as NonNullable<typeof session.user>
  vi.mocked(statisticsApi.activity).mockRejectedValue(new Error('offline'))
  recordPage('feed_open', 'school')
  session.user = { id: 'second' } as NonNullable<typeof session.user>
  await vi.advanceTimersByTimeAsync(1000)
  expect(statisticsApi.activity).toHaveBeenCalledTimes(1)
})
