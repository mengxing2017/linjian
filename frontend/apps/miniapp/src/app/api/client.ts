import { createClient } from '../../features/identity/api/contract'
import { createUniTransport } from '../transport'
import { createLegalDocumentsClient } from '../../features/legal-documents/api/contract'
export const client = createClient(
  createUniTransport(import.meta.env.VITE_LINJIAN_API_BASE_URL ?? ''),
)
export const legalDocuments = createLegalDocumentsClient(client)
