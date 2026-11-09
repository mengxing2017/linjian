import { requestResource, requestCollection, requestBinary } from '../../http'
import { createMiniappUsersClient } from './index'
export type { MiniappUser, UserFilters } from './index'
export const miniappUsers = createMiniappUsersClient({
  requestResource,
  requestCollection,
  requestBinary,
})
