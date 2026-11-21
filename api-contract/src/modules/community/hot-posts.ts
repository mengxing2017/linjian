import { decodeResource, invalidDocument, object, type ResourceDocument } from '../../http/protocol'

export interface HotPostSummary {
  id: string
  school_id: string
  rank: number
  title: string
  like_count: number
  comment_count: number
}
export interface HotPostsMeta {
  window_start: string
  window_end: string
  ranked_at: string
  validated_at: string
  limit: number
}
export interface HotPostsResult {
  items: HotPostSummary[]
  meta: HotPostsMeta
}
export interface HotPostsQuery {
  limit?: number
}

const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i
const timestamp = /^\d{4}-\d{2}-\d{2}T(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d(?:\.\d{1,9})?(?:Z|[+-](?:[01]\d|2[0-3]):[0-5]\d)$/
const windowMilliseconds = 7 * 24 * 60 * 60 * 1000
const count = (value: unknown): value is number => typeof value === 'number' && Number.isSafeInteger(value) && value >= 0
function instant(value: unknown): number {
  if (typeof value !== 'string' || !timestamp.test(value)) return invalidDocument()
  const parsed = Date.parse(value)
  const [year, month, day] = value.slice(0, 10).split('-').map(Number)
  const calendar = new Date(0)
  calendar.setUTCFullYear(year, month - 1, day)
  if (!Number.isFinite(parsed) || calendar.getUTCMonth() !== month - 1 || calendar.getUTCDate() !== day) return invalidDocument()
  return parsed
}

export function hotPostsLimit(query: HotPostsQuery): number {
  const limit = query.limit ?? 10
  if (!Number.isSafeInteger(limit) || limit < 1 || limit > 20) throw new Error('热榜条数必须为 1–20')
  return limit
}

export function readHotPosts(document: ResourceDocument, school: string, limit: number): HotPostsResult {
  if (!Array.isArray(document.data) || document.data.length > limit || !object(document.meta)) return invalidDocument()
  const meta = document.meta
  if (meta.limit !== limit) return invalidDocument()
  const end = instant(meta.window_end)
  if (end - instant(meta.window_start) !== windowMilliseconds || instant(meta.ranked_at) !== end || instant(meta.validated_at) !== end) return invalidDocument()
  const ids = new Set<string>()
  const items = document.data.map((resource, index) => {
    const value = decodeResource(resource)
    if (resource.type !== 'hot-posts' || !uuid.test(resource.id) || ids.has(resource.id.toLowerCase()) || value.school_id !== school || value.rank !== index + 1 || typeof value.title !== 'string' || !value.title.trim() || Array.from(value.title).length > 200 || !count(value.like_count) || !count(value.comment_count)) return invalidDocument()
    ids.add(resource.id.toLowerCase())
    return value as unknown as HotPostSummary
  })
  return { items, meta: meta as unknown as HotPostsMeta }
}
