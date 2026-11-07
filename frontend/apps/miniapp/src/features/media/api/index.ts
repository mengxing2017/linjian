import { createMediaClient } from '@linjian/api-contract/media'
import { client } from '../../identity/api'
export const mediaApi = createMediaClient(client)
