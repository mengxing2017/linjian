import { beforeEach, expect, it, vi } from 'vitest'
import { createPostSearch } from '../src/features/community/use-post-search'
import {
  invalidatePost,
  invalidatedPosts,
  clearPostInvalidations,
} from '../src/features/community/post-invalidations'
import type { CommunityPost } from '../src/features/community/api'

function post(id = 'post'): CommunityPost {
  return {
    id,
    school_id: 'school',
    author_id: 'user',
    title: '',
    content: '课本',
    display_label: '课本',
    images: [],
    tags: [],
    audit_status: 'approved',
    visibility: 'visible',
    revision: 1,
    display_published_at: '2026-11-24T00:00:00Z',
    published_at_text: '2026-11-24 08:00',
    author_name: '校园成员',
    like_count: 0,
    comment_count: 0,
  }
}
function result(ids = ['post'], number = 1, more = false) {
  return { items: ids.map((id) => post(id)), page: { number, size: 30, has_more: more } }
}
function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (cause: unknown) => void
  const promise = new Promise<T>((done, fail) => {
    resolve = done
    reject = fail
  })
  return { promise, resolve, reject }
}
async function setup() {
  let scope = { key: 'scope:user', school: 'school', user: 'user' }
  const api = { posts: vi.fn().mockResolvedValue(result()), interactionState: vi.fn() }
  const ensure = vi.fn().mockResolvedValue(true)
  const failure = vi.fn((cause: unknown) => (cause instanceof Error ? cause.message : '请求失败'))
  const search = createPostSearch({ api, scope: () => scope, ensure, failure })
  await search.show()
  search.draftKeyword.value = '课本'
  return {
    search,
    api,
    ensure,
    failure,
    setScope: (next: typeof scope) => {
      scope = next
      search.scopeChanged()
    },
  }
}
beforeEach(clearPostInvalidations)

it('waits for a valid explicit submission and counts Unicode scalars', async () => {
  const { search, api } = await setup()
  expect(api.posts).not.toHaveBeenCalled()
  await search.refresh()
  await search.loadMore()
  search.draftKeyword.value = '　'
  await search.submit()
  expect(search.inputError.value).toBe('请输入搜索关键词')
  search.draftKeyword.value = '😀'.repeat(101)
  await search.submit()
  expect(api.posts).not.toHaveBeenCalled()
  search.draftKeyword.value = ' 课本 '
  await search.submit()
  expect(api.posts).toHaveBeenCalledWith('school', { search: '课本', page: 1 }, expect.anything())
  expect(search.posts.value).toHaveLength(1)
})
it('uses the submitted snapshot for pagination, deduplicates and retries the same failed page', async () => {
  const { search, api } = await setup()
  api.posts.mockResolvedValueOnce(result(['one'], 1, true))
  await search.submit()
  search.draftKeyword.value = '未提交'
  api.posts.mockRejectedValueOnce(new Error('断网'))
  await search.loadMore()
  expect(search.page.value).toBe(1)
  expect(search.posts.value[0].id).toBe('one')
  expect(search.moreError.value).toBe('断网')
  api.posts.mockResolvedValueOnce(result(['one', 'two', 'two'], 2))
  await search.loadMore()
  expect(api.posts.mock.calls.at(-1)?.[1]).toEqual({ search: '课本', page: 2 })
  expect(search.posts.value.map((item) => item.id)).toEqual(['one', 'two'])
  await search.loadMore()
  expect(api.posts).toHaveBeenCalledTimes(3)
})
it('lets B supersede A and prevents stale success, failure and finally from changing B', async () => {
  const { search, api, failure } = await setup()
  const first = deferred<ReturnType<typeof result>>()
  const second = deferred<ReturnType<typeof result>>()
  api.posts.mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise)
  const requestA = search.submit()
  search.draftKeyword.value = 'B'
  const requestB = search.submit()
  expect(api.posts.mock.calls[0][2].aborted).toBe(true)
  first.reject(new Error('旧错误'))
  await requestA
  expect(search.loading.value).toBe(true)
  expect(failure).not.toHaveBeenCalled()
  second.resolve(result(['B']))
  await requestB
  expect(search.posts.value[0].id).toBe('B')
  expect(search.error.value).toBe('')
})
it('merges repeated submissions and reach-bottom while a request is pending', async () => {
  const { search, api } = await setup()
  const pending = deferred<ReturnType<typeof result>>()
  api.posts.mockReturnValueOnce(pending.promise)
  const first = search.submit()
  const repeat = search.submit()
  await search.loadMore()
  expect(api.posts).toHaveBeenCalledTimes(1)
  pending.resolve(result([], 1, true))
  await Promise.all([first, repeat])
  const next = deferred<ReturnType<typeof result>>()
  api.posts.mockReturnValueOnce(next.promise)
  const load = search.loadMore()
  await search.loadMore()
  expect(api.posts).toHaveBeenCalledTimes(2)
  next.resolve(result([], 2))
  await load
})
it('clears an active query without letting a late response restore it', async () => {
  const { search, api } = await setup()
  const pending = deferred<ReturnType<typeof result>>()
  api.posts.mockReturnValueOnce(pending.promise)
  const request = search.submit()
  search.clear()
  pending.resolve(result())
  await request
  expect(search.posts.value).toEqual([])
  expect(search.submittedKeyword.value).toBe('')
  expect(search.loading.value).toBe(false)
})
it('invalidates scope and clears another account’s draft, including pending responses', async () => {
  const { search, api, setScope } = await setup()
  const pending = deferred<ReturnType<typeof result>>()
  api.posts.mockReturnValueOnce(pending.promise)
  const request = search.submit()
  setScope({ key: 'next:user', school: 'next', user: 'user' })
  expect(search.draftKeyword.value).toBe('课本')
  expect(search.submittedKeyword.value).toBe('')
  pending.resolve(result())
  await request
  expect(search.posts.value).toEqual([])
  setScope({ key: 'next:other', school: 'next', user: 'other' })
  expect(search.draftKeyword.value).toBe('')
})
it('retains results on return and refreshes only the submitted query', async () => {
  const { search, api } = await setup()
  await search.submit()
  search.hide()
  await search.show()
  expect(api.posts).toHaveBeenCalledTimes(1)
  search.draftKeyword.value = '未提交'
  await search.refresh()
  expect(api.posts.mock.calls.at(-1)?.[1].search).toBe('课本')
  expect(search.draftKeyword.value).toBe('未提交')
})
it('uses read-only interaction state for invalidations without clearing unrelated or newer versions', async () => {
  const { search, api } = await setup()
  await search.submit()
  invalidatePost('scope:user', 'school', 'post')
  invalidatePost('scope:user', 'school', 'unrelated')
  api.interactionState.mockResolvedValue({ id: 'post', like_count: 3, comment_count: 2 })
  await search.sync()
  expect(search.posts.value[0].like_count).toBe(3)
  expect(invalidatedPosts('scope:user', 'school').map((item) => item.id)).toEqual(['unrelated'])
  invalidatePost('scope:user', 'school', 'post')
  api.interactionState.mockImplementationOnce(async () => {
    invalidatePost('scope:user', 'school', 'post', true)
    return { id: 'post', like_count: 99 }
  })
  await search.sync()
  expect(search.posts.value[0].like_count).toBe(3)
  expect(invalidatedPosts('scope:user', 'school').find((item) => item.id === 'post')?.content).toBe(
    true,
  )
})
it('requeries content changes and removes unreadable rows without a browsing request', async () => {
  const { search, api } = await setup()
  await search.submit()
  invalidatePost('scope:user', 'school', 'post', true)
  api.posts.mockResolvedValueOnce(result([]))
  await search.sync()
  expect(search.posts.value).toEqual([])
  expect(invalidatedPosts('scope:user', 'school')).toEqual([])
  await search.submit()
  invalidatePost('scope:user', 'school', 'post')
  api.interactionState.mockRejectedValueOnce({ status: 404 })
  await search.sync()
  expect(search.posts.value).toEqual([])
})
it('processes detail changes even when returning before an append request finishes', async () => {
  const { search, api } = await setup()
  api.posts.mockResolvedValueOnce(result(['post'], 1, true))
  await search.submit()
  const pending = deferred<ReturnType<typeof result>>()
  api.posts.mockReturnValueOnce(pending.promise)
  const append = search.loadMore()
  search.hide()
  invalidatePost('scope:user', 'school', 'post')
  api.interactionState.mockRejectedValueOnce({ status: 404 })
  await search.show()
  expect(api.interactionState).not.toHaveBeenCalled()
  pending.resolve(result(['another'], 2))
  await append
  await search.sync()
  expect(search.posts.value.map((item) => item.id)).toEqual(['another'])
  expect(api.interactionState).toHaveBeenCalledOnce()
})
it('distinguishes failure from no matches and rejects missing pagination', async () => {
  const { search, api } = await setup()
  api.posts.mockRejectedValueOnce(new Error('搜索失败'))
  await search.submit()
  expect(search.page.value).toBe(0)
  expect(search.error.value).toBe('搜索失败')
  api.posts.mockResolvedValueOnce(result([]))
  await search.refresh()
  expect(search.page.value).toBe(1)
  expect(search.error.value).toBe('')
  api.posts.mockResolvedValueOnce({ items: [] })
  await search.refresh()
  expect(search.error.value).toContain('分页')
})
it('does not resume or commit after disposal or a hidden pending show', async () => {
  const { search, api, ensure } = await setup()
  const entering = deferred<boolean>()
  ensure.mockReturnValueOnce(entering.promise)
  const show = search.show()
  search.hide()
  entering.resolve(true)
  await show
  expect(search.ready.value).toBe(false)
  const pending = deferred<ReturnType<typeof result>>()
  await search.show()
  api.posts.mockReturnValueOnce(pending.promise)
  const request = search.submit()
  search.dispose()
  pending.resolve(result())
  await request
  await search.show()
  expect(search.posts.value).toEqual([])
  expect(search.visible.value).toBe(false)
})
