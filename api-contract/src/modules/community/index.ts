import type { Client, Cancellation } from '../../http/client'
import { decodeResource, invalidDocument, type ResourceObject } from '../../http/protocol'
import { schoolPath } from '../schools'
import type { Post, Row } from '../business/types'
import { hotPostsLimit, readHotPosts, type HotPostsQuery } from './hot-posts'
export type { HotPostSummary, HotPostsMeta, HotPostsQuery, HotPostsResult } from './hot-posts'
export type { Post, Row } from '../business/types'
export interface CommunityPost extends Post {
  like_count: number
  comment_count: number
  author_name: string
}
export interface PostInteractionState {
  id: string
  school_id: string
  audit_status: Post['audit_status']
  visibility: Post['visibility']
  like_count: number
  comment_count: number
  liked: boolean
  collected: boolean
  author_followed: boolean
  can_interact: boolean
  can_share: boolean
}
function count(value: unknown) {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0
}
function readPost(resource: ResourceObject, school: string, id?: string): CommunityPost {
  const value = decodeResource(resource)
  if (resource.type !== 'posts' || value.school_id !== school || (id !== undefined && value.id !== id) || !count(value.like_count) || !count(value.comment_count))
    return invalidDocument()
  return value as unknown as CommunityPost
}
export interface PostInput {
  title: string
  content: string
  images: string[]
  tags: string[]
  candidate_tag_ids?: string[]
  revision?: number
}
export interface Comment extends Row {
  school_id: string
  post_id: string
  visibility: 'visible' | 'hidden'
  deleted: boolean
  created_at: string
  audit_status: 'pending' | 'approved' | 'rejected'
  author_id: string
  author_name: string
  content: string
  revision: number
  parent_id: string | null
}
export interface Reaction extends Row {
  post_id: string
  kind: 'like' | 'collect'
}
export interface Follow {
  id: string
  display_name: string
}
export interface Notification extends Row {
  kind: string
  object_id: string
  created_at: string
  read_at: string | null
}
export interface FeedFilter {
  page?: number
  search?: string
  mine?: boolean
  kind?: 'collect' | 'following'
  tag?: string
}
export type SchoolResource = import('../product-workflows/client').PublishedResource
export function pageQuery(page = 1) {
  return `page[number]=${page}&page[size]=20`
}
export function createCommunityClient(client: Client) {
  const scope = (s: string, p: string) => `${schoolPath(s)}/${p}`
  const part = encodeURIComponent
  return {
    async hotPosts(school: string, query: HotPostsQuery = {}, signal?: Cancellation) {
      const limit = hotPostsLimit(query)
      const document = await client.requestDocument(`${scope(school, 'hot-posts')}?limit=${limit}`, 'GET', undefined, signal)
      return readHotPosts(document, school, limit)
    },
    async posts(s: string, filter: FeedFilter = {}, signal?: Cancellation) {
      const query = `${pageQuery(filter.page)}&search=${part(filter.search ?? '')}&mine=${!!filter.mine}${filter.kind ? `&kind=${filter.kind}` : ''}&tag=${part(filter.tag ?? '')}`
      const doc = await client.requestDocument(scope(s, 'posts') + '?' + query, 'GET', undefined, signal)
      if (!Array.isArray(doc.data)) return invalidDocument()
      return { items: doc.data.map(resource => readPost(resource, s)), page: doc.meta?.page }
    },
    async post(s: string, id: string, signal?: Cancellation) {
      const doc = await client.requestDocument(scope(s, `posts/${part(id)}`), 'GET', undefined, signal)
      if (!doc.data || Array.isArray(doc.data)) return invalidDocument()
      return readPost(doc.data, s, id)
    },
    async interactionState(s: string, id: string, signal?: Cancellation): Promise<PostInteractionState> {
      const doc = await client.requestDocument(scope(s, `posts/${part(id)}/interaction-state`), 'GET', undefined, signal)
      if (!doc.data || Array.isArray(doc.data) || doc.data.type !== 'post-interaction-states') return invalidDocument()
      const value = decodeResource(doc.data)
      if (value.id !== id || value.school_id !== s || !count(value.like_count) || !count(value.comment_count)
        || !['pending', 'approved', 'rejected'].includes(String(value.audit_status))
        || !['visible', 'hidden', 'offline', 'deleted'].includes(String(value.visibility))
        || ['liked', 'collected', 'author_followed', 'can_interact', 'can_share'].some(key => typeof value[key] !== 'boolean'))
        return invalidDocument()
      return value as unknown as PostInteractionState
    },
    save: (s: string, input: PostInput, id?: string) =>
      client.requestResource<Post>(
        scope(s, id ? `posts/${part(id)}` : 'posts'),
        id ? 'PUT' : 'POST',
        input,
      ),
    remove: (s: string, post: Pick<Post, 'id' | 'revision'>) =>
      client.requestResource<Post>(scope(s, `posts/${part(post.id)}/visibility`), 'PUT', {
        revision: post.revision,
        visibility: 'deleted',
        top_sort: 0,
      }),
    comments: (s: string, id: string, page = 1) =>
      client.requestCollection<Comment>(
        scope(s, `posts/${part(id)}/comments`) + `?${pageQuery(page)}`,
      ),
    comment: (s: string, id: string, content: string, parent_id?: string) =>
      client.requestResource<Comment>(scope(s, `posts/${part(id)}/comments`), 'POST', {
        content,
        parent_id,
      }),
    deleteComment: (s: string, comment: Pick<Comment, 'id' | 'revision'>) =>
      client.requestResource<null>(scope(s, `comments/${part(comment.id)}`), 'DELETE', {
        revision: comment.revision,
      }),
    reactions: (s: string) => client.requestResource<Reaction[]>(scope(s, 'reactions')),
    react: (s: string, id: string, kind: 'like' | 'collect', active: boolean) =>
      client.requestResource<null>(
        scope(s, `posts/${part(id)}/reactions/${kind}`),
        active ? 'PUT' : 'DELETE',
      ),
    follows: (s: string) => client.requestResource<Follow[]>(scope(s, 'follows')),
    follow: (s: string, id: string, active: boolean) =>
      client.requestResource<null>(scope(s, `follows/${part(id)}`), active ? 'PUT' : 'DELETE'),
    notifications: (s: string, page = 1) =>
      client.requestCollection<Notification>(scope(s, 'notifications') + `?${pageQuery(page)}`),
    read: (s: string, id: string) =>
      client.requestResource<null>(scope(s, `notifications/${part(id)}/read`), 'PUT'),
    resources: (s: string) => client.requestResource<SchoolResource[]>(scope(s, 'published-resources')),
    click: (s: string, id: string) =>
      client.requestResource<null>(scope(s, `resources/${part(id)}/click`), 'POST'),
    report: (s: string, post_id: string, content: string) =>
      client.requestResource<Row>(scope(s, 'cases'), 'POST', {
        kind: 'report',
        post_id,
        content,
      }),
    feedback: (s: string, content: string, contact: string, images: string[] = []) =>
      client.requestResource<Row>(scope(s, 'cases'), 'POST', {
        kind: 'feedback',
        content,
        contact,
        images,
      }),
  }
}
