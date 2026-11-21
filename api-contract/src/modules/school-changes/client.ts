import type { Cancellation, Client } from '../../http/client'
import { invalidDocument } from '../../http/protocol'
import { readDocument, readPolicy, readRequest, readSummary } from './decode'
import type { SchoolChangeFilters, SchoolChangeRequest, SchoolChangeSubmission } from './types'

const own = '/api/v1/miniapp/school-change-requests'
const platform = '/api/v1/platform/school-change-requests'
const part = encodeURIComponent
export function createSchoolChangesClient(client: Pick<Client, 'requestDocument' | 'requestBinary'>) {
  const read = async (path: string, method = 'GET', body?: unknown, signal?: Cancellation) =>
    readRequest(await client.requestDocument(path, method, body, signal)) as SchoolChangeRequest
  return {
    policy: async () => readPolicy(await client.requestDocument('/api/v1/miniapp/school-change-policy')),
    current: async () => readRequest(await client.requestDocument(`${own}/current`), true),
    detail: (id: string, admin = false, signal?: Cancellation) => read(`${admin ? platform : own}/${part(id)}`, 'GET', undefined, signal),
    upload: async (bytes: ArrayBuffer) => readDocument(await client.requestDocument('/api/v1/miniapp/school-change-documents', 'POST', bytes)),
    submit: (input: SchoolChangeSubmission) => read(own, 'POST', input),
    cancel: (request: SchoolChangeRequest) => read(`${own}/${part(request.id)}/cancel`, 'POST', { revision: request.revision }),
    approve: (request: SchoolChangeRequest, reason: string) => read(`${platform}/${part(request.id)}/approve`, 'POST', { revision: request.revision, reason }),
    reject: (request: SchoolChangeRequest, reason: string) => read(`${platform}/${part(request.id)}/reject`, 'POST', { revision: request.revision, reason }),
    list: async (filters: SchoolChangeFilters = {}, signal?: Cancellation) => {
      const query: [string, string][] = [['page[number]', String(filters.page ?? 1)], ['page[size]', '30']]
      for (const key of ['status', 'account_id', 'created_from', 'created_to'] as const) if (filters[key]) query.push([key, filters[key]])
      const document = await client.requestDocument(`${platform}?${query.map(([key, value]) => `${part(key)}=${part(value)}`).join('&')}`, 'GET', undefined, signal)
      if (!Array.isArray(document.data) || !document.meta?.page) return invalidDocument()
      return { items: document.data.map(readSummary), page: document.meta.page }
    },
    document: (id: string, request?: string, signal?: Cancellation) => client.requestBinary(request
      ? `${platform}/${part(request)}/documents/${part(id)}`
      : `/api/v1/miniapp/school-change-documents/${part(id)}`, signal),
  }
}
