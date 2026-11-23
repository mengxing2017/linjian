import { expect, it, vi } from 'vitest'
import { readFileSync } from 'node:fs'
import { createMiniappUsersClient } from '../src/modules/miniapp-users'
import { createClient, writeDocument } from '../src/http/client'
const fixture = (name: string) => JSON.parse(readFileSync(new URL(`../fixtures/${name}.json`, import.meta.url), 'utf8'))

it('uses the same JSON:API binding resource for reads, authorization and versioned revocation', async () => {
  const document = fixture('miniapp-phone-binding')
  const transport = vi.fn().mockResolvedValue({ status: 200, headers: { 'content-type': 'application/vnd.api+json' }, body: document })
  const api = createMiniappUsersClient(createClient(transport))
  expect(await api.phoneBinding()).toEqual({ id: document.data.id, ...document.data.attributes })
  const input = fixture('miniapp-phone-binding-input')
  expect(writeDocument('/api/v1/miniapp/profile/phone-bindings', input.data.attributes)).toEqual(input)
  await api.bindPhone(input.data.attributes)
  expect(JSON.parse(transport.mock.calls[1]![0].body)).toEqual(input)
  await api.revokePhone(1)
  expect(transport.mock.calls[2]![0]).toMatchObject({ path: '/api/v1/miniapp/profile/phone-binding?revision=1', method: 'DELETE' })
  expect(transport.mock.calls[2]![0].body).toBeUndefined()
})

it('preserves the publication error code so clients do not discard the login or draft', async () => {
  const transport = vi.fn().mockResolvedValue({ status: 403, headers: { 'content-type': 'application/vnd.api+json' }, body: fixture('phone-verification-required') })
  await expect(createClient(transport).requestResource('/api/v1/miniapp/profile')).rejects.toMatchObject({ status: 403, code: 'phone_verification_required' })
})
