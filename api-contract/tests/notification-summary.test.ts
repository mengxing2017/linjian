import { readFileSync } from 'node:fs'
import { expect, it, vi } from 'vitest'
import { createClient } from '../src/http/client'
import { createCommunityClient } from '../src/modules/community'

const fixture = JSON.parse(readFileSync(new URL('../fixtures/notification-summary.json', import.meta.url), 'utf8'))
const school = fixture.data.id
function setup(body: unknown = fixture) {
  const transport = vi.fn().mockResolvedValue({ status: 200, headers: { 'content-type': 'application/vnd.api+json' }, body })
  return { transport, api: createCommunityClient(createClient(transport)) }
}
it('reads a school-scoped singleton and forwards cancellation without a write', async () => {
  const { api, transport } = setup()
  const signal = new AbortController().signal
  expect(await api.notificationSummary(school, signal)).toEqual({ id: school, school_id: school, unread_count: 103 })
  expect(transport).toHaveBeenCalledWith(expect.objectContaining({ method: 'GET', path: `/api/v1/schools/${school}/notification-summary`, body: undefined, signal }))
})
it('requires an authoritative nonnegative safe integer and matching school', async () => {
  for (const unread_count of [undefined, null, -1, 1.5, '12', Number.MAX_SAFE_INTEGER + 1]) {
    await expect(setup({ data: { ...fixture.data, attributes: { ...fixture.data.attributes, unread_count } } }).api.notificationSummary(school)).rejects.toMatchObject({ code: 'invalid_document' })
  }
  for (const data of [null, [], { ...fixture.data, id: 'other' }, { ...fixture.data, type: 'notifications' }, { ...fixture.data, attributes: { school_id: 'other', unread_count: 0 } }]) {
    await expect(setup({ data }).api.notificationSummary(school)).rejects.toMatchObject({ code: 'invalid_document' })
  }
  expect(await setup({ data: { ...fixture.data, attributes: { school_id: school, unread_count: 0 } } }).api.notificationSummary(school)).toMatchObject({ unread_count: 0 })
})
it('does not turn forbidden or unavailable responses into a zero count', async () => {
  const { api, transport } = setup()
  for (const status of [403, 404, 503]) {
    transport.mockResolvedValue({ status, headers: {}, body: { errors: [{ status: String(status), code: 'unavailable' }] } })
    await expect(api.notificationSummary(school)).rejects.toMatchObject({ status })
  }
})
