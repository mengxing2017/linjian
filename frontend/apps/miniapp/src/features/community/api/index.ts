export type {
  Post,
  CommunityPost,
  PostInteractionState,
  CreatePostInput,
  UpdatePostInput,
  Comment,
  Reaction,
  Follow,
  Notification,
  NotificationKind,
  NotificationFilter,
  NotificationSummary,
  FeedFilter,
  SchoolResource,
  HotPostSummary,
  HotPostsMeta,
  HotPostsResult,
} from '@linjian/api-contract/community'
export type { Cancellation } from '../../identity/api'
export type { HomeMenu, HomeMenuTag } from '@linjian/api-contract/home-menu'
export { communityApi, homeMenuApi } from './client'

export { normalizePostContent, postFieldError } from '@linjian/api-contract/community'
export { normalizePostSearch } from '@linjian/api-contract/community'
