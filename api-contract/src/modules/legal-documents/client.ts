import type { Client } from '../../http/client'
import type { LegalDocumentKind, LegalSummary, LegalDocument, LegalRevision } from './types'
export function createLegalDocumentsClient(client: Client) {
  const path = (kind: LegalDocumentKind) => `/api/v1/platform/legal-documents/${kind}`
  return {
    list: () => client.requestResource<LegalSummary[]>('/api/v1/legal-documents'),
    read: (kind: LegalDocumentKind) => client.requestResource<LegalDocument>(`/api/v1/legal-documents/${kind}`),
    admin: (kind: LegalDocumentKind) => client.requestResource<LegalDocument>(path(kind)),
    save: (kind: LegalDocumentKind, revision: number, content: string) => client.requestResource<LegalDocument>(path(kind), 'PUT', { revision, content }),
    clear: (kind: LegalDocumentKind, revision: number) => client.requestResource<LegalDocument>(`${path(kind)}/clear`, 'POST', { revision }),
    history: (kind: LegalDocumentKind, page = 1) => client.requestResource<LegalRevision[]>(`${path(kind)}/revisions?page=${page}`),
  }
}
