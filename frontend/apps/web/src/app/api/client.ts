import { client } from '../../features/identity/api/contract'
import { createMessageAdvertisementsClient } from '../../features/admin/message-advertisements/api/contract'

export { client }
export const messageAdvertisements = createMessageAdvertisementsClient(client)
