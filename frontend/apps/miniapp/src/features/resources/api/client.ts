import { createCommunityClient } from '@linjian/api-contract/community'
import { createHomeMenuClient } from '@linjian/api-contract/home-menu'
import { client } from '../../../app/api'
const community = createCommunityClient(client)
export const resourcesApi = { list: community.resources, click: community.click }
export const homeMenuApi = createHomeMenuClient(client)
