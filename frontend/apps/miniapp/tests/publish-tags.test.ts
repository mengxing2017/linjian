import { expect, it } from 'vitest'
import type { HomeMenuTag } from '../src/features/community/api'
import { emptyPublishTags, matchPublishTags, publishTagDraft, publishTagInput, restorePublishTags } from '../src/features/community/publish-tags'

const candidates: HomeMenuTag[] = [{ id: 'campus', name: '校园', menu_status: 'hot', source_type: 'admin', enable_status: 'enabled', sort_order: 0, revision: 1, created_at: '' }]

it('permits no tags, deduplicates legacy separators and serializes candidate IDs with their names', () => {
  expect(publishTagInput(emptyPublishTags())).toEqual({ tags: [], candidate_tag_ids: [] })
  const value = matchPublishTags(' 校园,校园，， ', candidates)
  expect(publishTagInput(value)).toEqual({ tags: ['校园'], candidate_tag_ids: ['campus'] })
  expect(restorePublishTags(publishTagDraft(value), candidates, [])).toEqual(value)
})

it('retains only original post names and explicitly blocks unresolved draft names', () => {
  const draft = { tags: '校园,旧标签，伪造历史标签' }
  const value = restorePublishTags(draft, candidates, ['旧标签'])
  expect(value).toEqual({ selected: [{ id: 'campus', name: '校园' }], retained: ['旧标签'], unresolved: ['伪造历史标签'] })
  expect(() => publishTagInput(value)).toThrow('待处理')
  expect(restorePublishTags(publishTagDraft(value), candidates, ['旧标签'])).toEqual(value)
  expect(restorePublishTags(draft, candidates, []).unresolved).toEqual(['旧标签', '伪造历史标签'])
})

it('does not silently reassign missing, renamed or cross-school IDs by name', () => {
  const value = matchPublishTags('校园', candidates)
  const draft = publishTagDraft(value)
  expect(restorePublishTags(draft, [{ ...candidates[0], id: 'other-school' }], []).unresolved).toEqual(['校园'])
  expect(restorePublishTags(draft, [{ ...candidates[0], name: '校园生活' }], []).unresolved).toEqual(['校园'])
  expect(restorePublishTags(draft, [], ['校园'])).toEqual({ selected: [], retained: ['校园'], unresolved: [] })
})

it('keeps unresolved names across repeated saves until explicitly removed', () => {
  let value = restorePublishTags({ tags: '校园,自定义' }, candidates, [])
  for (let index = 0; index < 3; index++) value = restorePublishTags(publishTagDraft(value), candidates, [])
  expect(value.unresolved).toEqual(['自定义'])
  value.unresolved = []
  expect(publishTagInput(value).tags).toEqual(['校园'])
})

it('counts retained tags toward the existing ten-tag limit', () => {
  const value = emptyPublishTags()
  value.retained = Array.from({ length: 10 }, (_, index) => `历史${index}`)
  expect(publishTagInput(value).tags).toHaveLength(10)
  value.selected = [{ id: 'campus', name: '校园' }]
  expect(() => publishTagInput(value)).toThrow('最多选择 10 个')
})

it('does not automatically resolve ambiguous duplicate catalog names', () => {
  expect(matchPublishTags('校园', [...candidates, { ...candidates[0], id: 'duplicate' }]).unresolved).toEqual(['校园'])
})
