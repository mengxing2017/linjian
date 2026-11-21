import { createMiniappUsersClient } from '@linjian/api-contract/miniapp-users'
import { client } from '../../../app/api'
export const miniappUserApi = createMiniappUsersClient(client)
