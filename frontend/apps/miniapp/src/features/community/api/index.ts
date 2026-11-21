import { createCommunityClient } from '@linjian/api-contract/community'
import { client } from '../../identity/api'
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
export const communityApi = createCommunityClient(client)

import { createHomeMenuClient } from '@linjian/api-contract/home-menu'
export type { HomeMenu, HomeMenuTag } from '@linjian/api-contract/home-menu'
export const homeMenuApi = createHomeMenuClient(client)
