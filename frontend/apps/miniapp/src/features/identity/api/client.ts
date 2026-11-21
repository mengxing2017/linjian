import { createIdentityClient } from './contract'
import { client } from '../../../app/api'
export const identityApi = createIdentityClient(client)
