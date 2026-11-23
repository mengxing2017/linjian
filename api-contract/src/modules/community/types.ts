import type { Post, Row } from '../business/types'
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
export type { CreatePostInput, UpdatePostInput } from './post-content'
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
