import type { HomeMenuTag } from './api'

export interface TagSnapshot { id: string; name: string }
export interface PublishTags {
  selected: TagSnapshot[]
  retained: string[]
  unresolved: string[]
}
export const emptyPublishTags = (): PublishTags => ({ selected: [], retained: [], unresolved: [] })
const names = (value: unknown): string[] => [...new Set(
  (typeof value === 'string' ? value.split(/[,，]/) : Array.isArray(value) ? value : [])
    .filter((item): item is string => typeof item === 'string').map(item => item.trim()).filter(Boolean),
)]
export function matchPublishTags(value: unknown, candidates: HomeMenuTag[], original: string[] = []): PublishTags {
  const result = emptyPublishTags()
  for (const name of names(value)) {
    const matches = candidates.filter(tag => tag.name === name)
    if (matches.length === 1) result.selected.push({ id: matches[0].id, name })
    else if (original.includes(name)) result.retained.push(name)
    else result.unresolved.push(name)
  }
  return result
}
export function restorePublishTags(draft: Record<string, unknown>, candidates: HomeMenuTag[], original: string[]): PublishTags {
  const saved = draft.tag_selection
  if (!saved || typeof saved !== 'object') return matchPublishTags(draft.tags, candidates, original)
  const value = saved as Record<string, unknown>
  const result = emptyPublishTags()
  if (Array.isArray(value.selected)) {
    for (const item of value.selected) {
      if (!item || typeof item !== 'object' || typeof item.name !== 'string') continue
      const name = item.name.trim()
      if (!name) continue
      const candidate = candidates.find(tag => tag.id === item.id && tag.name === name)
      if (candidate) result.selected.push({ id: candidate.id, name })
      else if (original.includes(name)) result.retained.push(name)
      else result.unresolved.push(name)
    }
  }
  for (const name of names(value.retained)) {
    const matched = matchPublishTags([name], candidates, original)
    result.selected.push(...matched.selected)
    result.retained.push(...matched.retained)
    result.unresolved.push(...matched.unresolved)
  }
  result.selected = result.selected.filter((tag, index, all) => all.findIndex(other => other.id === tag.id) === index)
  result.retained = names(result.retained).filter(name => !result.selected.some(tag => tag.name === name))
  result.unresolved = names([...result.unresolved, ...names(value.unresolved)])
  return result
}
export const publishTagNames = (value: PublishTags) => [...value.selected.map(tag => tag.name), ...value.retained]
export function publishTagInput(value: PublishTags) {
  if (value.unresolved.length) throw new Error('请先确认移除待处理的旧草稿标签')
  const tags = publishTagNames(value)
  if (tags.length > 10) throw new Error('最多选择 10 个标签')
  return { tags, candidate_tag_ids: value.selected.map(tag => tag.id) }
}
export function publishTagDraft(value: PublishTags) {
  return { schema_version: 3, tags: [...publishTagNames(value), ...value.unresolved].join(','), tag_selection: value }
}
