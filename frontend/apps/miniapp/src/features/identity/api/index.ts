import { createClient } from '@linjian/api-contract/client'
import { createIdentityClient } from '@linjian/api-contract/identity'
import { createUniTransport } from '../../../app/transport'
export type { Transport, TransportRequest, Cancellation } from '@linjian/api-contract/client'
export { ApiError } from '@linjian/api-contract/client'
export type { User } from '@linjian/api-contract/identity'
export const client = createClient(createUniTransport(import.meta.env.VITE_API_BASE_URL ?? ''))
export const identityApi = createIdentityClient(client)
