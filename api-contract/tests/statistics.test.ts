import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { createClient, type TransportRequest } from '../src/http/client'
import { createStatisticsClient } from '../src/modules/statistics'
const fixture = (name: string) => JSON.parse(readFileSync(new URL(`../fixtures/${name}.json`, import.meta.url), 'utf8'))
describe('business statistics contract', () => {
  it('decodes single-school and unavailable values without replacing null by zero', async () => {
    for (const name of ['school-statistics', 'statistics-unavailable']) {
      const doc = fixture(name)
      const api = createStatisticsClient(createClient(async () => ({ status: 200, headers: { 'content-type': 'application/vnd.api+json' }, body: doc })))
      const r = await api.report('school-a', { period: 'day', anchor_date: '2026-11-16' })
      expect(r).toEqual({ id: doc.data.id, ...doc.data.attributes })
      expect(r.summary.active_users).toBe(name === 'statistics-unavailable' ? null : 3)
    }
  })
  it('keeps platform query explicit and propagates cancellation', async () => {
    let request: TransportRequest | undefined
    const api = createStatisticsClient(createClient(async r => { request = r; return { status: 200, headers: { 'content-type': 'application/vnd.api+json' }, body: fixture('platform-statistics') } }))
    const signal = new AbortController().signal
    const r = await api.report(undefined, { period: 'month', school_ids: 'a,b', school_status: 'hidden' }, signal)
    expect(request?.path).toBe('/api/v1/platform/statistics?period=month&school_ids=a%2Cb&school_status=hidden')
    expect(request?.signal).toBe(signal)
    expect(r.registration_groups.unknown).toBe(1)
  })
  it('encodes miniapp events as their own JSON:API type and exports the original snapshot', async () => {
    const requests: TransportRequest[] = []
    const api = createStatisticsClient(createClient(async r => { requests.push(r); return { status: 202, headers: { 'content-type': 'application/vnd.api+json' }, body: r.binary ? new ArrayBuffer(3) : { data: { type: 'miniapp-activity-events', id: 'event', attributes: { accepted: true } } } } }))
    await api.activity({ event_id: 'event', kind: 'feed_open', school_id: 'school' })
    expect(JSON.parse(requests[0].body as string).data.type).toBe('miniapp-activity-events')
    await api.export('school', 'snapshot')
    expect(requests[1].path).toBe('/api/v1/schools/school/statistics/snapshots/snapshot/export?kind=periods')
    expect(requests[1].binary).toBe(true)
  })
})
