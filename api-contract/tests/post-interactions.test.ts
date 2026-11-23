import { readFileSync } from 'node:fs'
import { expect, it, vi } from 'vitest'
import { createClient } from '../src/http/client'
import { createCommunityClient } from '../src/modules/community'

const fixture = JSON.parse(readFileSync(new URL('../fixtures/post-interaction-state.json', import.meta.url), 'utf8'))
const school = fixture.data.attributes.school_id
const post = fixture.data.id
function setup(body: unknown = fixture) {
  const transport = vi.fn().mockResolvedValue({ status: 200, headers: { 'content-type': 'application/vnd.api+json' }, body })
  return { transport, api: createCommunityClient(createClient(transport)) }
}
it('reads the school-scoped JSON:API snapshot and forwards cancellation', async () => {
  const { api, transport } = setup()
  const signal = new AbortController().signal
  expect(await api.interactionState(school, post, signal)).toEqual({ id: post, ...fixture.data.attributes })
  expect(transport).toHaveBeenCalledWith(expect.objectContaining({ path: `/api/v1/schools/${school}/posts/${post}/interaction-state`, method: 'GET', signal }))
})
it('rejects mismatched resources and missing or invalid required fields without fallback', async () => {
  for (const patch of [{ like_count: undefined }, { comment_count: -1 }, { like_count: 0.5 }, { liked: 1 }, { can_share: undefined }, { school_id: 'other' }, { visibility: 'unknown' }]) {
    const { api } = setup({ data: { ...fixture.data, attributes: { ...fixture.data.attributes, ...patch } } })
    await expect(api.interactionState(school, post)).rejects.toMatchObject({ code: 'invalid_document' })
  }
  for (const patch of [{ type: 'posts' }, { id: 'other' }]) {
    await expect(setup({ data: { ...fixture.data, ...patch } }).api.interactionState(school, post)).rejects.toMatchObject({ code: 'invalid_document' })
  }
})
it('does not decode errors or cancelled reads as zero counts', async () => {
  const { api, transport } = setup()
  transport.mockResolvedValue({ status: 403, body: { errors: [{ code: 'forbidden', detail: '无权限' }] }, headers: {} })
  await expect(api.interactionState(school, post)).rejects.toMatchObject({ status: 403 })
  const controller = new AbortController()
  controller.abort()
  await expect(api.interactionState(school, post, controller.signal)).rejects.toMatchObject({ code: 'cancelled' })
})
it('requires counters on community reads rather than accepting an old post shape', async () => {
  for (const attributes of [{ school_id: school }, { school_id: school, like_count: 0, comment_count: -1 }]) {
    await expect(setup({ data: { type: 'posts', id: post, attributes } }).api.post(school, post)).rejects.toMatchObject({ code: 'invalid_document' })
  }
  const attributes = { school_id: school, title: '', display_label: '正文', like_count: 0, comment_count: 0 }
  expect(await setup({ data: { type: 'posts', id: post, attributes } }).api.post(school, post)).toMatchObject(attributes)
})
it('decodes the community post and distinguishes pending comments from approved replies', async () => {
  const postFixture = JSON.parse(readFileSync(new URL('../fixtures/post-community.json', import.meta.url), 'utf8'))
  expect(await setup(postFixture).api.post(school, post)).toMatchObject({ like_count: 0, comment_count: 0 })
  const pending = JSON.parse(readFileSync(new URL('../fixtures/comment-pending.json', import.meta.url), 'utf8'))
  const reply = JSON.parse(readFileSync(new URL('../fixtures/comment-approved-reply.json', import.meta.url), 'utf8'))
  const result = await setup({ data: [pending.data, reply.data], meta: { page: { number: 1, size: 20, has_more: false } } }).api.comments(school, post)
  expect(result.items[0]).toMatchObject({ audit_status: 'pending', parent_id: null })
  expect(result.items[1]).toMatchObject({ audit_status: 'approved', parent_id: reply.data.attributes.parent_id })
})
