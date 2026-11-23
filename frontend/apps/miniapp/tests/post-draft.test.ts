import { expect, it } from 'vitest'
import { postDraftKey } from '../src/features/community/post-draft'

it('uses only the new draft namespace and isolates account, school and edited post', () => {
  const key = postDraftKey('account', 'school')
  expect(key).toBe('linjian:draft:v2:account:school:new')
  expect(new Set([
    key,
    postDraftKey('another', 'school'),
    postDraftKey('account', 'another'),
    postDraftKey('account', 'school', 'post'),
  ]).size).toBe(4)
  expect(key).not.toBe('linjian:draft:account:school:new')
})
