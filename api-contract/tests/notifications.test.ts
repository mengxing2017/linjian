import { readFileSync } from 'node:fs'
import { expect, it, vi } from 'vitest'
import { createClient } from '../src/http/client'
import { createCommunityClient, type NotificationKind } from '../src/modules/community'

const fixture = (name: string) => JSON.parse(readFileSync(new URL(`../fixtures/notifications-${name}.json`, import.meta.url), 'utf8'))
const school = '00000000-0000-4000-8000-000000000001'
function setup(body: unknown = fixture('all')) {
  const transport = vi.fn().mockResolvedValue({ status: 200, headers: { 'content-type': 'application/vnd.api+json' }, body })
  return { transport, api: createCommunityClient(createClient(transport)) }
}

it('preserves old calls and appends only an explicit validated notification kind', async () => {
  const { api, transport } = setup()
  await api.notifications(school)
  expect(transport).toHaveBeenLastCalledWith(expect.objectContaining({ method: 'GET', path: `/api/v1/schools/${school}/notifications?page[number]=1&page[size]=20`, body: undefined }))
  await api.notifications(school, 3)
  expect(transport.mock.lastCall![0].path).toContain('page[number]=3&page[size]=20')
  for (const kind of ['comment', 'like', 'follow', 'moderation'] as const) {
    await api.notifications(school, 2, { kind })
    expect(transport.mock.lastCall![0].path).toBe(`/api/v1/schools/${school}/notifications?page[number]=2&page[size]=20&kind=${kind}`)
  }
  expect(transport.mock.calls.every(([request]) => request.method === 'GET')).toBe(true)
})

it('rejects malformed kinds before transport without treating them as all', async () => {
  const { api, transport } = setup()
  for (const kind of ['', ' ', 'Comment', 'all', 'like&mine=true', null]) {
    await expect(api.notifications(school, 1, { kind: kind as NotificationKind })).rejects.toThrow('消息分类无效')
  }
  expect(transport).not.toHaveBeenCalled()
})

it('decodes shared fixtures, preserves pagination and unknown response kinds', async () => {
  for (const name of ['all', 'comment', 'moderation', 'empty']) {
    const document = fixture(name)
    const result = await setup(document).api.notifications(school)
    expect(result.items).toEqual(document.data.map((resource: { id: string; attributes: object }) => ({ id: resource.id, ...resource.attributes })))
    expect(result.page).toEqual(document.meta.page)
  }
  expect((await setup().api.notifications(school)).items[1].kind).toBe('future')
  await expect(setup({ data: null }).api.notifications(school)).rejects.toMatchObject({ code: 'invalid_document' })
})

it('preserves JSON:API validation and authorization errors', async () => {
  const { api, transport } = setup()
  for (const [status, code] of [[422, 'validation'], [403, 'forbidden'], [503, 'unavailable']] as const) {
    transport.mockResolvedValue({ status, headers: {}, body: { errors: [{ status: String(status), code, detail: '请求失败' }] } })
    await expect(api.notifications(school, 1, { kind: 'comment' })).rejects.toMatchObject({ status, code, message: '请求失败' })
  }
})
