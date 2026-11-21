import type { Client, Cancellation } from '../../http/client'

export interface RegistrationSettings {
  nickname_prefix: string
  revision: number
  updated_at: string
  updated_by_name: string | null
}

export interface RegistrationSettingsInput {
  nickname_prefix: string
  revision: number
}

export function createRegistrationSettingsClient(client: Pick<Client, 'requestResource'>) {
  const path = '/api/v1/platform/miniapp-registration-settings'
  return {
    read: (signal?: Cancellation) =>
      client.requestResource<RegistrationSettings>(path, 'GET', undefined, signal),
    update: (input: RegistrationSettingsInput) =>
      client.requestResource<RegistrationSettings>(path, 'PUT', input),
  }
}
