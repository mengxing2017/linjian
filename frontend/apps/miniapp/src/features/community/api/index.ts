export type {
  Post,
  CommunityPost,
  PostInteractionState,
  PostInput,
  Comment,
  Reaction,
  Follow,
  Notification,
  FeedFilter,
  SchoolResource,
  HotPostSummary,
  HotPostsMeta,
  HotPostsResult,
} from '@linjian/api-contract/community'
export type { Cancellation } from '../../identity/api'
export type { HomeMenu, HomeMenuTag } from '@linjian/api-contract/home-menu'
export { communityApi, homeMenuApi } from './client'
