import { createMediaClient } from '@linjian/api-contract/media'
import { client } from '../../../app/api'
export const mediaApi = createMediaClient(client)
