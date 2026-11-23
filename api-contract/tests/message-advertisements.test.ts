import { expect, it, vi } from 'vitest'
import { readFileSync } from 'node:fs'
import { createClient } from '../src/http/client'
import { createMessageAdvertisementsClient, advertisementContentError, normalizeAdvertisement } from '../src/modules/message-advertisements'
import { readConfiguration, readRevision } from '../src/modules/message-advertisements/decode'

const fixture = (name: string) => JSON.parse(readFileSync(new URL(`../fixtures/message-advertisement-${name}.json`, import.meta.url), 'utf8'))
const configuration = () => {
  const data = fixture('current').data
  return readConfiguration({ ...data.attributes, id: data.id })
}
it('shares Unicode normalization and scalar-count fixtures with Rust', () => {
  for (const sample of fixture('rules').normalization) {
    if (sample.normalized === null) expect(advertisementContentError(sample.input)).not.toBeNull()
    else { expect(normalizeAdvertisement(sample.input)).toBe(sample.normalized); expect(advertisementContentError(sample.input)).toBeNull() }
  }
  expect(advertisementContentError('\ud800')).not.toBeNull()
})
it('decodes draft, pending, approved and rejected evidence without rounding versions', () => {
  for (const name of ['draft', 'pending', 'current']) {
    const data = fixture(name).data
    expect(readConfiguration({ ...data.attributes, id: data.id }).school_id).toBeTruthy()
  }
  const data = fixture('rejected').data
  expect(readRevision({ ...data.attributes, id: data.id }).decision?.reason).toContain('广告标识')
  expect(() => readConfiguration({ ...configuration(), version: 3 })).toThrow()
  expect(readConfiguration({ ...configuration(), version: '9007199254740994' }).version).toBe('9007199254740994')
  expect(() => readConfiguration({ ...configuration(), current_revision: { ...configuration().current_revision, content: '坏\n文案' } })).toThrow()
})
it('uses typed JSON:API write actions and sends exact review versions and digest', async () => {
  const transport = vi.fn().mockResolvedValue({ status: 200, headers: { 'content-type': 'application/vnd.api+json' }, body: fixture('current') })
  const api = createMessageAdvertisementsClient(createClient(transport))
  const config = configuration(), revision = config.current_revision!
  await api.create(config.school_id, '广告')
  await api.saveDraft(config.school_id, config, '新版广告')
  await api.submit(config.school_id, config, revision)
  await api.withdraw(config.school_id, config, revision)
  await api.decide(revision, 'reject', '请修改')
  await api.remove(config.school_id, config)
  await api.removePlatform(config, '不合适')
  const documents = transport.mock.calls.map(([request]) => JSON.parse(request.body))
  expect(documents.map(document => document.data.type)).toEqual([
    'school-message-advertisements', 'school-message-advertisement-drafts', 'school-message-advertisement-submissions',
    'school-message-advertisement-withdrawals', 'school-message-advertisement-decisions', 'school-message-advertisements', 'school-message-advertisement-removals',
  ])
  expect(documents[4].data.attributes).toEqual({ expected_config_version: '3', expected_revision_version: '3', review_digest: 'a'.repeat(64), decision: 'reject', reason: '请修改' })
  expect(transport.mock.calls[5][0].method).toBe('DELETE')
  expect(documents[5].data.attributes.expected_config_version).toBe('3')
})
it('handles empty configuration and forwards cancellation and server filters', async () => {
  const transport = vi.fn().mockResolvedValueOnce({ status: 200, headers: { 'content-type': 'application/vnd.api+json' }, body: fixture('empty') })
    .mockResolvedValue({ status: 200, headers: { 'content-type': 'application/vnd.api+json' }, body: { data: [], meta: { page: { number: 2, size: 30, has_more: false } } } })
  const api = createMessageAdvertisementsClient(createClient(transport))
  const controller = new AbortController()
  expect(await api.current('school', controller.signal)).toBeNull()
  await api.list({ school_id: 'school', status: 'rejected', page: 2 }, controller.signal)
  const request = transport.mock.calls[1][0]
  expect(request.signal).toBe(controller.signal)
  const url = new URL(request.path, 'https://local.example')
  expect(url.searchParams.get('school_id')).toBe('school')
  expect(url.searchParams.get('status')).toBe('rejected')
  expect(url.searchParams.get('page[number]')).toBe('2')
})
