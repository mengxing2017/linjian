import { expect, it, vi } from 'vitest'
import { createFileCache } from '../src/shared/file-cache'
import { mapConcurrent } from '../src/shared/concurrency'
import { mergePageWindow, previousWindow, windowStart } from '../src/shared/page-window'
import { editorText, draftWithinLimits } from '../src/features/community/draft-limits'
import { feedExcerpt } from '../src/features/community/post-feed-card'

it('bounds page groups and allows every earlier group to be loaded again', () => {
  let items: { id: string }[] = []
  for (let page = 1; page <= 30; page++) {
    const incoming = Array.from({ length: 30 }, (_, index) => ({ id: `${page}:${index}` }))
    items = mergePageWindow(items, page - 1, incoming, page)
    expect(items.length).toBeLessThanOrEqual(90)
    expect(items[0].id).toBe(`${windowStart(page)}:0`)
  }
  expect(previousWindow(30)).toBe(25)
  expect(previousWindow(4)).toBe(1)
  items = mergePageWindow(items, 30, [{ id: '25:0' }], 25)
  expect(items).toEqual([{ id: '25:0' }])
  expect(mergePageWindow([{ id: 'same', value: 1 }], 1, [{ id: 'same', value: 2 }], 2))
    .toEqual([{ id: 'same', value: 2 }])
})

it('limits summaries and editor text without splitting Unicode surrogate pairs', () => {
  expect(feedExcerpt('😀'.repeat(241))).toBe(`${'😀'.repeat(240)}…`)
  expect(editorText('😀'.repeat(201), 200)).toBe('😀'.repeat(200))
  expect(draftWithinLimits('😀'.repeat(200), '😀'.repeat(20000))).toBe(true)
  expect(draftWithinLimits('title', 'text'.repeat(20000))).toBe(false)
  expect(draftWithinLimits('a'.repeat(201), 'body')).toBe(false)
})

it('deduplicates files, evicts only unreferenced LRU files and respects byte budgets', async () => {
  const remove = vi.fn(), cache = createFileCache(remove, 2, 10)
  const owner = {}, another = {}
  const read = (key: string, bytes: number, pin?: object) => cache.read(key, async () => ({ path: key, bytes }), pin)
  const pending = read('pinned', 6, owner)
  expect(read('pinned', 6, another)).toBe(pending)
  await pending
  await read('unused', 6)
  expect(remove).not.toHaveBeenCalledWith('unused')
  expect(remove).not.toHaveBeenCalledWith('pinned')
  cache.release(owner)
  expect(remove).toHaveBeenCalledWith('unused')
  await read('active', 6, owner)
  expect(remove).not.toHaveBeenCalledWith('pinned')
  cache.release(another)
  expect(remove).toHaveBeenCalledWith('pinned')
  cache.clear()
  expect(remove).toHaveBeenCalledWith('active')
})

it('evicts by recency and permits retry after a failed file load', async () => {
  const remove = vi.fn(), cache = createFileCache(remove, 2)
  const load = (key: string) => cache.read(key, async () => ({ path: key, bytes: 1 }))
  await load('old')
  await load('recent')
  await load('old')
  await load('new')
  expect(remove).toHaveBeenCalledExactlyOnceWith('recent')
  await expect(cache.read('retry', async () => { throw new Error('offline') })).rejects.toThrow('offline')
  await expect(load('retry')).resolves.toBe('retry')
})

it('cleans up late file writes after scope invalidation without poisoning new loads', async () => {
  const remove = vi.fn(), cache = createFileCache(remove)
  let complete!: (file: { path: string; bytes: number }) => void
  const old = cache.read('same', () => new Promise(resolve => { complete = resolve }))
  cache.clear()
  const fresh = cache.read('same', async () => ({ path: 'fresh', bytes: 1 }))
  complete({ path: 'stale', bytes: 1 })
  await expect(old).rejects.toThrow('资源范围已改变')
  await expect(fresh).resolves.toBe('fresh')
  expect(remove).toHaveBeenCalledExactlyOnceWith('stale')
  expect(cache.read('same', vi.fn())).toBe(fresh)
})

it('limits parallel work, preserves result order and stops scheduling after cancellation', async () => {
  let active = 0, peak = 0
  const result = await mapConcurrent([1, 2, 3, 4, 5, 6, 7], async value => {
    active++
    peak = Math.max(peak, active)
    await Promise.resolve()
    active--
    return value * 2
  })
  expect(peak).toBe(3)
  expect(result).toEqual([2, 4, 6, 8, 10, 12, 14])
  let current = true
  const read = vi.fn(async value => { current = false; return value })
  await mapConcurrent([1, 2, 3, 4], read, () => current)
  expect(read).toHaveBeenCalledOnce()
})
