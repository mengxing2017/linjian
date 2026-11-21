import { requestBinary, requestCollection, requestResource } from '../../http/browser-client'
import { createMiniappUsersClient } from './client'
import { createRegistrationSettingsClient } from './registration-settings'
export type { RegistrationSettings, RegistrationSettingsInput } from './registration-settings'
export type { MiniappUser, UserFilters } from './types'
export const registrationSettings = createRegistrationSettingsClient({ requestResource })
export const miniappUsers = createMiniappUsersClient({
  requestResource,
  requestCollection,
  requestBinary,
})
