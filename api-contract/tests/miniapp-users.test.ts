import { expect, it, vi } from 'vitest'
import { readFileSync } from 'node:fs'
import { createMiniappUsersClient } from '../src/modules/miniapp-users'
import { createClient, writeDocument } from '../src/http/client'
const fixture = (name: string) =>
  JSON.parse(
    readFileSync(new URL(`../fixtures/${name}.json`, import.meta.url), 'utf8'),
  )
it('uses school-scoped user resources and independent profile/status action types', async () => {
  const user = fixture('miniapp-user')
  const transport = vi.fn().mockResolvedValue({
    status: 200,
    headers: { 'content-type': 'application/vnd.api+json' },
    body: user,
  })
  const api = createMiniappUsersClient(createClient(transport))
  const result = await api.detail('school-1', user.data.id)
  expect(result.nickname).toBe('校园用户')
  expect(result).not.toHaveProperty('username')
  await api.status('school-1', result, 'banned', '违反学校规则')
  expect(JSON.parse(transport.mock.calls[1][0].body)).toEqual(
    fixture('miniapp-user-status-input'),
  )
  expect(
    writeDocument(
      '/api/v1/miniapp/profile',
      fixture('miniapp-user-profile-input').data.attributes,
    ),
  ).toEqual(fixture('miniapp-user-profile-input'))
  expect(
    writeDocument(
      '/api/v1/miniapp/profile/mobile',
      fixture('miniapp-user-mobile-input').data.attributes,
    ),
  ).toEqual(fixture('miniapp-user-mobile-input'))
  expect(
    writeDocument(
      '/api/v1/platform/miniapp-users/user-1/school',
      fixture('miniapp-user-school-input').data.attributes,
    ),
  ).toEqual(fixture('miniapp-user-school-input'))
  await expect(async () =>
    api.avatar('https://evil.example/avatar'),
  ).rejects.toThrow('头像地址无效')
})
it('preserves server pagination and encodes filters without overriding a path scope', async () => {
  const transport = vi.fn().mockResolvedValue({
    status: 200,
    headers: { 'content-type': 'application/vnd.api+json' },
    body: {
      data: [],
      meta: { page: { number: 2, size: 30, has_more: false } },
    },
  })
  const api = createMiniappUsersClient(createClient(transport))
  expect(
    (
      await api.list('school-1', {
        page: 2,
        search: '昵称 &',
        status: 'normal',
        sort: '-post_count',
      })
    ).page?.number,
  ).toBe(2)
  expect(transport.mock.calls[0][0].path).toContain(
    '/schools/school-1/miniapp-users?',
  )
  expect(transport.mock.calls[0][0].path).toContain(
    'search=%E6%98%B5%E7%A7%B0%20%26',
  )
})
it('sends mobile searches in the request body rather than the URL', async () => {
  const transport = vi.fn().mockResolvedValue({
    status: 200,
    headers: { 'content-type': 'application/vnd.api+json' },
    body: { data: [] },
  })
  await createMiniappUsersClient(createClient(transport)).list('school-1', {
    mobile: '13800138000',
    page: 2,
  })
  const request = transport.mock.calls[0][0]
  expect(request.path).toBe('/api/v1/schools/school-1/miniapp-users/searches')
  expect(request.method).toBe('POST')
  expect(JSON.parse(request.body)).toEqual({
    data: {
      type: 'miniapp-user-searches',
      attributes: {
        'page[number]': 2,
        'page[size]': 30,
        mobile: '13800138000',
      },
    },
  })
})
