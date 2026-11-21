import { requestResource, requestCollection, requestBinary } from '../../http'
import { createMiniappUsersClient } from './index'
import { createRegistrationSettingsClient } from './registration-settings'
export type { RegistrationSettings, RegistrationSettingsInput } from './registration-settings'
export const registrationSettings = createRegistrationSettingsClient({ requestResource })
export type { MiniappUser, UserFilters } from './index'
export const miniappUsers = createMiniappUsersClient({
  requestResource,
  requestCollection,
  requestBinary,
})
