export interface BlockedWord {
  id: string
  school_id: string | null
  word: string
  remark: string
  active: boolean
  revision: number
  sort_order: number
}
export interface BlockedWordCatalog {
  id: string
  items: BlockedWord[]
  order_revision: number
  has_more: boolean
}
export interface BlockedWordInput {
  word: string
  remark: string
  active: boolean
  revision?: number
}
export interface BlockedWordImport {
  id: string
  digest: string
  rows: Array<{ row: number; word: string; remark: string; active: boolean }>
  errors: Array<{ row: number; message: string }>
  expires_at: string
}
export function createBlockedWordsClient(client: import('../../http/client').Client) {
  const path = (school?: string) =>
    school
      ? `/api/v1/schools/${encodeURIComponent(school)}/blocked-words`
      : '/api/v1/platform/blocked-words'
  return {
    list: (
      school: string | undefined,
      global: boolean,
      search: string,
      status: string,
      page: number,
      ordering: boolean,
    ) =>
      client.requestResource<BlockedWordCatalog>(
        path(school) +
          '?' +
          new URLSearchParams({
            global: String(global),
            search,
            status: status === 'all' ? '' : status,
            page: String(page),
            ordering: String(ordering),
          }),
      ),
    save: (school: string | undefined, input: BlockedWordInput, id?: string) =>
      client.requestResource<BlockedWord>(
        path(school) + (id ? `/${encodeURIComponent(id)}` : ''),
        id ? 'PUT' : 'POST',
        input,
      ),
    remove: (school: string | undefined, id: string, revision: number) =>
      client.requestResource<null>(`${path(school)}/${encodeURIComponent(id)}`, 'DELETE', {
        revision,
      }),
    order: (school: string | undefined, ids: string[], order_revision: number) =>
      client.requestResource<null>(`${path(school)}/order`, 'POST', { ids, order_revision }),
    commitImport: (school: string | undefined, id: string, digest: string) =>
      client.requestResource<{ id: string; count: number }>(
        `${school ? `/api/v1/schools/${encodeURIComponent(school)}` : '/api/v1/platform'}/blocked-word-imports/${encodeURIComponent(id)}/commit`,
        'POST',
        { digest },
      ),
  }
}
