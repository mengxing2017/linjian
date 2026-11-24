import { readFileSync } from 'node:fs'
import { expect, it, vi } from 'vitest'
import { createClient } from '../src/http/client'
import { createCommunityClient, normalizePostSearch } from '../src/modules/community'
import cases from '../fixtures/post-search-normalization.json'

const post = JSON.parse(
  readFileSync(new URL('../fixtures/post-community.json', import.meta.url), 'utf8'),
).data
const document = {
  data: [post],
  meta: { page: { number: 2, size: 30, has_more: false } },
}
const school = post.attributes.school_id
function setup(body: unknown = document) {
  const transport = vi.fn().mockResolvedValue({
    status: 200,
    headers: { 'content-type': 'application/vnd.api+json' },
    body,
  })
  return { transport, api: createCommunityClient(createClient(transport)) }
}

it('normalizes the same scalar and whitespace rules as Rust', () => {
  for (const fixture of cases) expect(normalizePostSearch(fixture.input)).toBe(fixture.output)
  expect(normalizePostSearch()).toBe('')
  expect(normalizePostSearch('😀'.repeat(100))).toBe('😀'.repeat(100))
  expect(() => normalizePostSearch('😀'.repeat(101))).toThrow('搜索关键词最多 100 字')
})
it('encodes literal text within the current school and preserves JSON:API pagination', async () => {
  const { api, transport } = setup()
  const signal = new AbortController().signal
  const result = await api.posts(school, { search: ' 课本 & !%_\\ ', page: 2 }, signal)
  const request = transport.mock.calls[0][0]
  const url = new URL(request.path, 'https://linjian.test')
  expect(url.pathname).toBe(`/api/v1/schools/${school}/posts`)
  expect(url.searchParams.get('search')).toBe('课本 & !%_\\')
  expect(url.searchParams.get('page[number]')).toBe('2')
  expect(url.searchParams.get('mine')).toBe('false')
  expect(url.searchParams.has('kind')).toBe(false)
  expect(request.signal).toBe(signal)
  expect(result.items[0]).toEqual({ id: post.id, ...post.attributes })
  expect(result.page).toEqual(document.meta.page)
})
it('keeps empty list queries and personal filters compatible', async () => {
  const { api, transport } = setup()
  await api.posts(school, {
    search: '　',
    mine: true,
    kind: 'collect',
    tag: '校园',
  })
  const url = new URL(transport.mock.calls[0][0].path, 'https://linjian.test')
  expect(url.searchParams.get('search')).toBe('')
  expect(url.searchParams.get('mine')).toBe('true')
  expect(url.searchParams.get('kind')).toBe('collect')
  expect(url.searchParams.get('tag')).toBe('校园')
  await expect(api.posts(school, { search: '字'.repeat(101) })).rejects.toThrow(
    '搜索关键词最多 100 字',
  )
  expect(transport).toHaveBeenCalledTimes(1)
})
it('rejects malformed pagination and another school instead of producing an empty result', async () => {
  for (const page of [null, {}, { number: 1, size: 30, has_more: 'yes' }]) {
    await expect(setup({ data: [post], meta: { page } }).api.posts(school)).rejects.toMatchObject({
      code: 'invalid_document',
    })
  }
  await expect(setup().api.posts('other')).rejects.toMatchObject({
    code: 'invalid_document',
  })
})
