import { beforeEach, expect, it, vi } from 'vitest'
import { acknowledgePost, clearPostInvalidations, invalidatePost, invalidatedPosts, onPostInvalidated } from '../src/features/community/post-invalidations'
beforeEach(clearPostInvalidations)
it('broadcasts scope-local invalidations without consuming the feed queue, and unsubscribes cleanly', () => {
  const listener = vi.fn()
  const stop = onPostInvalidated(listener)
  invalidatePost('account', 'school', 'post')
  expect(listener).toHaveBeenCalledWith({ scope: 'account', school: 'school', id: 'post' })
  expect(invalidatedPosts('account', 'school')).toHaveLength(1)
  clearPostInvalidations()
  expect(listener).toHaveBeenCalledOnce()
  stop()
  invalidatePost('account', 'school', 'another')
  expect(listener).toHaveBeenCalledOnce()
})
it('isolates accounts and schools and acknowledges only the exact invalidation version', () => {
  invalidatePost('account-a', 'school-a', 'post')
  const first = invalidatedPosts('account-a', 'school-a')[0]
  expect(invalidatedPosts('account-b', 'school-a')).toEqual([])
  expect(invalidatedPosts('account-a', 'school-b')).toEqual([])
  invalidatePost('account-a', 'school-a', 'post')
  acknowledgePost('account-a', 'school-a', 'post', first.version)
  expect(invalidatedPosts('account-a', 'school-a')).toHaveLength(1)
  const latest = invalidatedPosts('account-a', 'school-a')[0]
  acknowledgePost('account-a', 'school-a', 'post', latest.version)
  expect(invalidatedPosts('account-a', 'school-a')).toEqual([])
})
it('retains content invalidation across count updates without pretending content changed again', () => {
  invalidatePost('account', 'school', 'post', true)
  const first = invalidatedPosts('account', 'school')[0]
  invalidatePost('account', 'school', 'post')
  const next = invalidatedPosts('account', 'school')[0]
  expect(next.content).toBe(true)
  expect(next.contentVersion).toBe(first.contentVersion)
  expect(next.version).toBeGreaterThan(first.version)
  clearPostInvalidations()
  expect(invalidatedPosts('account', 'school')).toEqual([])
})
