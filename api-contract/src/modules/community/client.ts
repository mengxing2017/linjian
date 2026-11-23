import type { Cancellation, Client } from '../../http/client'
import { invalidDocument } from '../../http/protocol'
import { schoolPath } from '../schools/path'
import { hotPostsLimit, readHotPosts, type HotPostsQuery } from './hot-posts'
import { pageQuery } from './query'
import type {
  Comment,
  FeedFilter,
  Follow,
  Notification,
  Post,
  CreatePostInput,
  UpdatePostInput,
  PostInteractionState,
  Reaction,
  Row,
  SchoolResource,
} from './types'
import { readInteractionState, readPost } from './validation'
import { normalizePostContent } from './post-content'
export function createCommunityClient(client: Client) {
  const scope = (s: string, p: string) => `${schoolPath(s)}/${p}`
  const part = encodeURIComponent
  return {
    async hotPosts(school: string, query: HotPostsQuery = {}, signal?: Cancellation) {
      const limit = hotPostsLimit(query)
      const document = await client.requestDocument(
        `${scope(school, 'hot-posts')}?limit=${limit}`,
        'GET',
        undefined,
        signal,
      )
      return readHotPosts(document, school, limit)
    },
    async posts(s: string, filter: FeedFilter = {}, signal?: Cancellation) {
      const query = `${pageQuery(filter.page)}&search=${part(filter.search ?? '')}&mine=${!!filter.mine}${filter.kind ? `&kind=${filter.kind}` : ''}&tag=${part(filter.tag ?? '')}`
      const doc = await client.requestDocument(
        scope(s, 'posts') + '?' + query,
        'GET',
        undefined,
        signal,
      )
      if (!Array.isArray(doc.data)) return invalidDocument()
      return { items: doc.data.map((resource) => readPost(resource, s)), page: doc.meta?.page }
    },
    async post(s: string, id: string, signal?: Cancellation) {
      const doc = await client.requestDocument(
        scope(s, `posts/${part(id)}`),
        'GET',
        undefined,
        signal,
      )
      if (!doc.data || Array.isArray(doc.data)) return invalidDocument()
      return readPost(doc.data, s, id)
    },
    async interactionState(
      s: string,
      id: string,
      signal?: Cancellation,
    ): Promise<PostInteractionState> {
      const doc = await client.requestDocument(
        scope(s, `posts/${part(id)}/interaction-state`),
        'GET',
        undefined,
        signal,
      )
      return readInteractionState(doc.data, s, id)
    },
    create: (s: string, input: CreatePostInput) =>
      client.requestResource<Post>(
        scope(s, 'posts'),
        'POST',
        { ...input, ...normalizePostContent(input) },
      ),
    update: (s: string, id: string, input: UpdatePostInput) =>
      client.requestResource<Post>(
        scope(s, `posts/${part(id)}`),
        'PUT',
        { ...input, ...normalizePostContent(input) },
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
    resources: (s: string) =>
      client.requestResource<SchoolResource[]>(scope(s, 'published-resources')),
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
