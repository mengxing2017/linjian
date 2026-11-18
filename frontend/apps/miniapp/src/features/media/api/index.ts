import { createMediaClient } from '@linjian/api-contract/media'
import { client } from '../../identity/api'
export const mediaApi = createMediaClient(client)

export { uploadKey, type UploadPurpose } from '@linjian/api-contract/media'
