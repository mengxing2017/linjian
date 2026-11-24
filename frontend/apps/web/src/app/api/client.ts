import { client } from '../../features/identity/api/contract'
import { createLegalDocumentsClient } from '../../features/admin/legal-documents/api/contract'
export const legalDocuments = createLegalDocumentsClient(client)
import { createMessageAdvertisementsClient } from '../../features/admin/message-advertisements/api/contract'

export { client }
export const messageAdvertisements = createMessageAdvertisementsClient(client)
