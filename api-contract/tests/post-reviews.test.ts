import { afterEach, expect, it, vi } from 'vitest'
import { readFileSync } from 'node:fs'
import { workspace } from '../src/modules/business'
import { resourceType, writeDocument } from '../src/http/protocol'
afterEach(() => vi.unstubAllGlobals())
it('reads a complete paginated review and serializes the full review decision', async () => {
  const fixture = JSON.parse(readFileSync(new URL('../fixtures/post-review.json', import.meta.url), 'utf8'))
  const fetch = vi.fn().mockResolvedValue(Response.json({data:[fixture.data],meta:{page:{number:1,size:1,has_more:true}}}, {headers:{'content-type':'application/vnd.api+json'}}))
  vi.stubGlobal('fetch',fetch)
  const result = await workspace.reviews('school-1','?page%5Bsize%5D=1&status=pending')
  expect(result.items[0]).toMatchObject(fixture.data.attributes)
  expect(result.items[0].content).toContain('正文末尾')
  expect(result.page?.has_more).toBe(true)
  expect(resourceType('/api/v1/schools/school-1/posts/post-1/review')).toBe('post-reviews')
  const decision = JSON.parse(readFileSync(new URL('../fixtures/post-review-decision-input.json', import.meta.url), 'utf8'))
  expect(writeDocument('/api/v1/schools/school-1/posts/post-1/moderation-decisions',decision.data.attributes)).toEqual(decision)
  expect(decision.data.attributes).not.toHaveProperty('source_preview_digest')
})

it.each(['human', 'automatic', 'legacy'])('reads %s review evidence without inventing an operator', async (source) => {
  const fixture = JSON.parse(readFileSync(new URL(`../fixtures/post-review-${source}.json`, import.meta.url), 'utf8'))
  vi.stubGlobal('fetch', vi.fn().mockResolvedValue(Response.json(fixture, { headers: { 'content-type': 'application/vnd.api+json' } })))
  const review = await workspace.review('school-1', 'post-1')
  expect(review.decision_source).toBe(source)
  expect(review.moderator_id !== null).toBe(source === 'human')
})
