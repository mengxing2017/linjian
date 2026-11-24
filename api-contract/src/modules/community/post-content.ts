export interface PostContent {
  title: string
  content: string
  images: string[]
  tags: string[]
  candidate_tag_ids?: string[]
}
export type CreatePostInput = Omit<PostContent, 'title'> & { title?: string }
export type UpdatePostInput = PostContent & { revision: number }

const invisible = /[\s\u0000-\u001f\u007f-\u009f\u200b-\u200d\u2060\ufeff\ufe00-\ufe0f\u{e0100}-\u{e01ef}]/gu
const leadingTrailingSpace = /^[\u0009-\u000d\u0020\u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]+|[\u0009-\u000d\u0020\u0085\u00a0\u1680\u2000-\u200a\u2028\u2029\u202f\u205f\u3000]+$/g
const singleLineForbidden = /[\u0000-\u001f\u007f-\u009f\u2028\u2029]/
const trimText = (value: string) => value.replace(leadingTrailingSpace, '')

export function postFieldError(field: 'title' | 'content', input: string): string | undefined {
  const value = trimText(input)
  const meaningful = value.replace(invisible, '').length > 0
  if (field === 'title') {
    if (singleLineForbidden.test(value)) return '标题必须为单行文本'
    if (Array.from(value).length > 200) return '标题最多 200 字'
    if (value && !meaningful) return '标题不能仅包含不可见字符'
  } else {
    if (!meaningful) return '请填写正文'
    if (Array.from(value).length > 20000) return '正文最多 20000 字'
  }
}

export function normalizePostContent(input: { title?: string; content: string }) {
  const title = input.title ?? ''
  for (const field of ['title', 'content'] as const) {
    const error = postFieldError(field, field === 'title' ? title : input.content)
    if (error) throw new Error(error)
  }
  return { title: trimText(title), content: trimText(input.content) }
}
