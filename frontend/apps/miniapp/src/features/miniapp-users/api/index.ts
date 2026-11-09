import { createMiniappUsersClient } from '@linjian/api-contract/miniapp-users'
import { client } from '../../identity/api'
export type { MiniappUser } from '@linjian/api-contract/miniapp-users'
export const miniappUserApi = createMiniappUsersClient(client)
