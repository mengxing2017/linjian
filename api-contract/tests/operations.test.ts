import { expect, it, vi } from 'vitest'
import { readFileSync } from 'node:fs'
import { createClient } from '../src/http/client'
import { createOperationsClient } from '../src/modules/operations'
it('reads a hidden display and sends metric IDs and revisioned deletions as JSON:API', async () => {
  const body = JSON.parse(readFileSync(new URL('../fixtures/operations-display.json', import.meta.url), 'utf8'))
  const transport = vi.fn().mockResolvedValueOnce({ status: 200, headers: { 'content-type': 'application/vnd.api+json' }, body })
    .mockResolvedValue({ status: 200, headers: { 'content-type': 'application/vnd.api+json' }, body: { data: null } })
  const api = createOperationsClient(createClient(transport))
  expect((await api.display('school')).customer.enabled).toBe(false)
  await api.deleteFeedback('school', 'feedback-id', 3)
  expect(JSON.parse(transport.mock.calls[1][0].body)).toEqual({ data: { type: 'feedback', attributes: { revision: 3 } } })
  const event = { event_id: 'event', resource_id: 'ad', event_type: 'view' as const, display_cycle: 'cycle', occurred_at: '2026-11-10T09:00:00Z' }
  await api.event('school', event)
  expect(JSON.parse(transport.mock.calls[2][0].body)).toEqual({ data: { type: 'banner-events', attributes: event } })
  await api.saveCustomer({ revision: 0, enabled: false, business_asset_id: null, join_asset_id: null, service_asset_id: null, fallback_asset_id: null, note: '' })
  expect(JSON.parse(transport.mock.calls[3][0].body).data.type).toBe('customer-config')
})
it('decodes defaults, customer state and historical feedback without inventing absent metadata', async () => {
  const fixture = (name: string) => JSON.parse(readFileSync(new URL(`../fixtures/operations-${name}.json`, import.meta.url), 'utf8'))
  const transport = vi.fn().mockResolvedValueOnce({ status: 200, headers: { 'content-type': 'application/vnd.api+json' }, body: fixture('qrcodes') })
    .mockResolvedValueOnce({ status: 200, headers: { 'content-type': 'application/vnd.api+json' }, body: fixture('customer') })
    .mockResolvedValueOnce({ status: 200, headers: { 'content-type': 'application/vnd.api+json' }, body: fixture('feedback') })
  const api = createOperationsClient(createClient(transport))
  const rows = await api.list('school', 'group-qrcodes', { defaults: true, page: 1, size: 20 })
  expect(rows.items[0]).toMatchObject({ school_id: null, popup_strategy: 'daily', popup_interval_days: 1 })
  expect(transport.mock.calls[0][0].path).toContain('defaults=true')
  expect(rows.page?.has_more).toBe(false)
  expect(await api.customer()).toMatchObject({ enabled: false, revision: 0, fallback_asset_id: null })
  expect((await api.feedback('school')).items[0]).toMatchObject({ feedback_type: null, submitted_ip: null, state: 'dismissed' })
})
