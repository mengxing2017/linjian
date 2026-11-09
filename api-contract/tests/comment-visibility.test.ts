import { afterEach, expect, it, vi } from 'vitest'
import { readFileSync } from 'node:fs'
import { workspace } from '../src/modules/business'
import { setCsrf } from '../src/http'
afterEach(() => { vi.unstubAllGlobals(); setCsrf('') })
it('writes independent comment visibility with its revision and decodes the JSON:API response', async () => {
  const fixture = JSON.parse(readFileSync(new URL('../fixtures/comment-hidden.json', import.meta.url), 'utf8'))
  const fetch = vi.fn().mockResolvedValue(Response.json(fixture, { headers: { 'content-type': 'application/vnd.api+json' } }))
  vi.stubGlobal('fetch', fetch)
  setCsrf('comment-csrf')
  const result = await workspace.setCommentVisibility('school-1', 'comment-1', { revision: 1, visibility: 'hidden' })
  expect(result).toMatchObject({ id: 'comment-1', audit_status: 'approved', visibility: 'hidden', revision: 2 })
  const [url, options] = fetch.mock.calls[0]
  expect(url).toBe('/api/v1/schools/school-1/comments/comment-1/visibility')
  expect(options.method).toBe('PUT')
  const input = JSON.parse(readFileSync(new URL('../fixtures/comment-visibility-input.json', import.meta.url), 'utf8'))
  expect(JSON.parse(options.body)).toEqual(input)
})
