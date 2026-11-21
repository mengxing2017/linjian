import { expect, it, vi } from 'vitest'
import { readFileSync } from 'node:fs'
import { createClient } from '../src/http/client'
import { createRegistrationSettingsClient } from '../src/modules/miniapp-users/registration-settings'

const fixture = (name: string) => JSON.parse(readFileSync(new URL(`../fixtures/${name}.json`, import.meta.url), 'utf8'))

it('uses a platform-only singleton and the same resource type for writes', async () => {
  const transport = vi.fn().mockResolvedValue({ status: 200, headers: { 'content-type': 'application/vnd.api+json' }, body: fixture('miniapp-registration-settings') })
  const api = createRegistrationSettingsClient(createClient(transport))
  expect(await api.read()).toMatchObject({ nickname_prefix: '用户', revision: 0, updated_by_name: null })
  await api.update({ nickname_prefix: '校园用户', revision: 0 })
  const request = transport.mock.calls[1][0]
  expect(request.path).toBe('/api/v1/platform/miniapp-registration-settings')
  expect(request.method).toBe('PUT')
  expect(JSON.parse(request.body)).toEqual(fixture('miniapp-registration-settings-input'))
})

it('preserves configuration conflicts instead of retrying writes', async () => {
  const transport = vi.fn().mockResolvedValue({ status: 409, headers: { 'content-type': 'application/vnd.api+json' }, body: { errors: [{ code: 'conflict', detail: '配置已变化' }] } })
  await expect(createRegistrationSettingsClient(createClient(transport)).update({ nickname_prefix: '用户', revision: 0 }))
    .rejects.toMatchObject({ status: 409, code: 'conflict' })
  expect(transport).toHaveBeenCalledOnce()
})
