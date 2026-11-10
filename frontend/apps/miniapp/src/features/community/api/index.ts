import { createCommunityClient } from '@linjian/api-contract/community'
import { client } from '../../identity/api'
export type {
  Post,
  PostInput,
  Comment,
  Reaction,
  Follow,
  Notification,
  FeedFilter,
  SchoolResource,
} from '@linjian/api-contract/community'
export const communityApi = createCommunityClient(client)

import { createHomeMenuClient } from '@linjian/api-contract/home-menu'
export type { HomeMenu, HomeMenuTag } from '@linjian/api-contract/home-menu'
export const homeMenuApi = createHomeMenuClient(client)
