import { createCommunityClient } from '@linjian/api-contract/community'
import { createHomeMenuClient } from '@linjian/api-contract/home-menu'
import { client } from '../../../app/api'
export const communityApi = createCommunityClient(client)
export const homeMenuApi = createHomeMenuClient(client)
