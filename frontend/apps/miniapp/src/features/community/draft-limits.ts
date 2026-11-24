export const DRAFT_TITLE_LIMIT = 200
export const DRAFT_CONTENT_LIMIT = 20000

export function editorText(value: string, limit: number) {
  if (value.length <= limit) return value
  let count = 0
  let end = 0
  for (const character of value) {
    if (count === limit) break
    end += character.length
    count++
  }
  return value.slice(0, end)
}

export function draftWithinLimits(title: string, content: string) {
  return editorText(title, DRAFT_TITLE_LIMIT).length === title.length &&
    editorText(content, DRAFT_CONTENT_LIMIT).length === content.length
}
