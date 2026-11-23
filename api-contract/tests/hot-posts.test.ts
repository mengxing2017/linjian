import { readFileSync } from 'node:fs'
import { expect, it, vi } from 'vitest'
import { createClient } from '../src/http/client'
import { createCommunityClient } from '../src/modules/community'

const fixture = JSON.parse(readFileSync(new URL('../fixtures/hot-posts.json', import.meta.url), 'utf8'))
const empty = JSON.parse(readFileSync(new URL('../fixtures/hot-posts-empty.json', import.meta.url), 'utf8'))
const school = fixture.data[0].attributes.school_id
function setup(body: unknown = fixture) {
  const transport = vi.fn().mockResolvedValue({ status: 200, headers: { 'content-type': 'application/vnd.api+json' }, body })
  return { transport, api: createCommunityClient(createClient(transport)) }
}
it('reads typed hot summaries and mandatory metadata, and forwards cancellation without a write document', async () => {
  const { api, transport } = setup()
  const signal = new AbortController().signal
  const result = await api.hotPosts(school, {}, signal)
  expect(result).toEqual({ items: [{ id: fixture.data[0].id, ...fixture.data[0].attributes }], meta: fixture.meta })
  expect(transport).toHaveBeenCalledWith(expect.objectContaining({ path: `/api/v1/schools/${school}/hot-posts?limit=10`, method: 'GET', signal, body: undefined }))
  expect(await setup(empty).api.hotPosts(school)).toEqual({ items: [], meta: empty.meta })
  await setup({ ...empty, meta: { ...empty.meta, limit: 20 } }).api.hotPosts(school, { limit: 20 })
})
it('rejects malformed items rather than falling back to community post fields or zero counts', async () => {
  for (const patch of [
    { school_id: 'other' }, { rank: 0 }, { rank: 2 }, { display_label: '' }, { display_label: ' '.repeat(5) },
    { like_count: undefined }, { like_count: -1 }, { comment_count: 0.5 }, { like_count: Number.MAX_SAFE_INTEGER + 1 },
  ]) {
    const body = { ...fixture, data: [{ ...fixture.data[0], attributes: { ...fixture.data[0].attributes, ...patch } }] }
    await expect(setup(body).api.hotPosts(school)).rejects.toMatchObject({ code: 'invalid_document' })
  }
  for (const patch of [{ type: 'posts' }, { id: 'not-a-uuid' }]) {
    await expect(setup({ ...fixture, data: [{ ...fixture.data[0], ...patch }] }).api.hotPosts(school)).rejects.toMatchObject({ code: 'invalid_document' })
  }
  const duplicate = { ...fixture.data[0], attributes: { ...fixture.data[0].attributes, rank: 2 } }
  await expect(setup({ ...fixture, data: [fixture.data[0], duplicate] }).api.hotPosts(school)).rejects.toMatchObject({ code: 'invalid_document' })
  const tooMany = { ...fixture, data: Array.from({ length: 11 }, () => fixture.data[0]) }
  await expect(setup(tooMany).api.hotPosts(school)).rejects.toMatchObject({ code: 'invalid_document' })
  await expect(setup({ data: [{ post_id: fixture.data[0].id, display_label: '旧结构' }] }).api.hotPosts(school)).rejects.toMatchObject({ code: 'invalid_document' })
})
it('requires valid and consistent ranking metadata, not a cached time or client-clock comparison', async () => {
  for (const patch of [
    { limit: undefined }, { limit: 20 }, { window_start: '2026-11-13T04:00:00Z' },
    { window_end: 'not-a-date' }, { ranked_at: '2026-11-21T04:00:01Z' },
    { validated_at: undefined }, { window_start: '2026-02-30T04:00:00Z' },
    { ranked_at: '2026-11-21T24:00:00Z' },
  ]) {
    await expect(setup({ ...fixture, meta: { ...fixture.meta, ...patch } }).api.hotPosts(school)).rejects.toMatchObject({ code: 'invalid_document' })
  }
  await expect(setup({ data: fixture.data }).api.hotPosts(school)).rejects.toMatchObject({ code: 'invalid_document' })
  const offsets = { ...fixture, meta: { ...fixture.meta, ranked_at: '2026-11-21T12:00:00+08:00' } }
  expect((await setup(offsets).api.hotPosts(school)).items).toHaveLength(1)
  const unicode = { ...fixture, data: [{ ...fixture.data[0], attributes: { ...fixture.data[0].attributes, display_label: '🌟'.repeat(80) } }] }
  expect((await setup(unicode).api.hotPosts(school)).items).toHaveLength(1)
})
it('rejects invalid local limits, HTTP failures and cancellation without turning them into empty lists', async () => {
  for (const limit of [0, 21, -1, 1.2, NaN]) {
    const { api, transport } = setup()
    await expect(api.hotPosts(school, { limit })).rejects.toThrow()
    expect(transport).not.toHaveBeenCalled()
  }
  for (const status of [401, 403, 404, 503]) {
    const { api, transport } = setup()
    transport.mockResolvedValue({ status, headers: {}, body: { errors: [{ code: 'failure' }] } })
    await expect(api.hotPosts(school)).rejects.toMatchObject({ status })
  }
  const controller = new AbortController()
  controller.abort()
  await expect(setup().api.hotPosts(school, {}, controller.signal)).rejects.toMatchObject({ code: 'cancelled' })
})
