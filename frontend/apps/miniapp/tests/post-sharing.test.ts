import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { postShare } from '../src/features/community/sharing'
import type { PostInteractionState } from '../src/features/community/api'

const state: PostInteractionState = { id: 'post', school_id: 'school', audit_status: 'approved', visibility: 'visible', like_count: 0, comment_count: 0, liked: false, collected: false, author_followed: false, can_interact: true, can_share: true }
it('shares only the permitted route with neutral copy and a bundled static cover', () => {
  expect(postShare(state, 'school', 'post', true)).toEqual({ title: '在 Linjian 查看帖子', path: '/pages/index/detail?school=school&id=post', imageUrl: '/static/images/share-cover.png' })
  const png = readFileSync(new URL('../src/static/images/share-cover.png', import.meta.url))
  expect([...png.subarray(0, 8)]).toEqual([137, 80, 78, 71, 13, 10, 26, 10])
  expect(png.readUInt32BE(16)).toBe(500)
  expect(png.readUInt32BE(20)).toBe(400)
})
it('never reuses stale or unauthorized post data in a callback', () => {
  for (const target of [undefined, { ...state, can_share: false }, { ...state, id: 'other' }, { ...state, school_id: 'other' }, { ...state, audit_status: 'pending' as const }, { ...state, visibility: 'hidden' as const }])
    expect(postShare(target, 'school', 'post', true)).toMatchObject({ title: 'Linjian', path: '/pages/index/index' })
  expect(postShare(state, 'school', 'post', false)).toMatchObject({ path: '/pages/index/index' })
})
