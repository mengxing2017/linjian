import { createCommunityClient } from '@linjian/api-contract/community'
import { client } from '../../identity/api'
export type { SchoolResource } from '@linjian/api-contract/community'
const community = createCommunityClient(client)
export const resourcesApi = { list: community.resources, click: community.click }
