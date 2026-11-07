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
