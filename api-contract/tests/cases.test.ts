import { afterEach, expect, it, vi } from 'vitest'
import { readFileSync } from 'node:fs'
import { workspace } from '../src/modules/business'
afterEach(() => vi.unstubAllGlobals())
const fixture = (name: string) => JSON.parse(readFileSync(new URL(`../fixtures/case-${name}.json`, import.meta.url), 'utf8'))
it.each(['open', 'resolved', 'dismissed', 'feedback', 'legacy'])('reads the shared %s case without fabricating metadata', async name => {
  const document = fixture(name)
  const fetch = vi.fn().mockResolvedValue(Response.json({ data: [document.data], meta: { page: { number: 2, size: 1, has_more: false } } }, { headers: { 'content-type': 'application/vnd.api+json' } }))
  vi.stubGlobal('fetch', fetch)
  const signal = new AbortController().signal
  const query = `?${new URLSearchParams({ 'page[number]': '2', 'page[size]': '1', status: document.data.attributes.state, search: '举报人 昵称' })}`
  const result = await workspace.cases('school a', query, signal)
  expect(result.items[0]).toEqual({ id: document.data.id, ...document.data.attributes })
  expect(result.page?.number).toBe(2)
  expect(fetch.mock.calls[0][0]).toBe(`/api/v1/schools/school%20a/cases${query}`)
  if (name === 'legacy') expect(result.items[0].handled_at).toBeNull()
})
it('encodes case identity and writes only handling input in JSON:API', async () => {
  const fetch = vi.fn().mockResolvedValue(Response.json(fixture('resolved'), { headers: { 'content-type': 'application/vnd.api+json' } }))
  vi.stubGlobal('fetch', fetch)
  const input = { revision: 1, state: 'resolved' as const, resolution: '核实处理' }
  const result = await workspace.resolveCase('school a', 'case a', input)
  expect(result.handled_at).not.toBeNull()
  expect(fetch.mock.calls[0][0]).toBe('/api/v1/schools/school%20a/cases/case%20a')
  expect(fetch.mock.calls[0][1].method).toBe('PUT')
  expect(JSON.parse(fetch.mock.calls[0][1].body)).toEqual({ data: { type: 'cases', attributes: input } })
})

it('keeps the transport path traversal guard', async () => {
  await expect(workspace.cases('school/other')).rejects.toThrow('API path must be same-origin')
})
