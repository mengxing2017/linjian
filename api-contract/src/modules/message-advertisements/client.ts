import type { Cancellation, Client } from '../../http/client'
import { decodeResource, invalidDocument } from '../../http/protocol'
import { readConfiguration, readRevision, readSingle } from './decode'
import type { AdvertisementFilters, AdvertisementRevision, MessageAdvertisement } from './types'

export function createMessageAdvertisementsClient(client: Pick<Client, 'requestDocument'>) {
  const part = encodeURIComponent
  const root = (school: string) => `/api/v1/schools/${part(school)}/message-advertisements`
  const configKind = 'school-message-advertisements'
  const revisionKind = 'school-message-advertisement-revisions'
  const write = async (path: string, method: string, input: unknown) =>
    readConfiguration(readSingle(await client.requestDocument(path, method, input), configKind))
  const collection = async (path: string, signal?: Cancellation) => {
    const document = await client.requestDocument(path, 'GET', undefined, signal)
    if (!Array.isArray(document.data) || !document.meta?.page) return invalidDocument()
    const items = document.data.map(item => {
      if (item.type !== revisionKind) return invalidDocument()
      return readRevision(decodeResource(item))
    })
    return { items, page: document.meta.page }
  }
  const action = (school: string, config: MessageAdvertisement, revision: AdvertisementRevision, name: string) =>
    write(`${root(school)}/${part(config.id)}/revisions/${part(revision.id)}/${name}`, 'POST', {
      expected_config_version: config.version, expected_revision_version: revision.version,
    })
  return {
    current: async (school: string, signal?: Cancellation) => {
      const value = readSingle(await client.requestDocument(`${root(school)}/current`, 'GET', undefined, signal), configKind)
      return value === null ? null : readConfiguration(value)
    },
    create: (school: string, content: string) => write(root(school), 'POST', { content }),
    saveDraft: (school: string, config: MessageAdvertisement, content: string) => write(`${root(school)}/${part(config.id)}/draft`, 'PUT', {
      content, expected_config_version: config.version, expected_revision_version: config.candidate?.version ?? null,
    }),
    submit: (school: string, config: MessageAdvertisement, revision: AdvertisementRevision) => action(school, config, revision, 'submissions'),
    withdraw: (school: string, config: MessageAdvertisement, revision: AdvertisementRevision) => action(school, config, revision, 'withdrawals'),
    remove: (school: string, config: MessageAdvertisement) => write(`${root(school)}/${part(config.id)}`, 'DELETE', { expected_config_version: config.version }),
    history: (school: string, advertisement: string, page = 1, signal?: Cancellation) => collection(`${root(school)}/${part(advertisement)}/revisions?page%5Bnumber%5D=${page}`, signal),
    detail: async (revision: string, signal?: Cancellation) => readRevision(readSingle(await client.requestDocument(`/api/v1/platform/message-advertisement-revisions/${part(revision)}`, 'GET', undefined, signal), revisionKind)),
    schoolDetail: async (school: string, advertisement: string, revision: string, signal?: Cancellation) => readRevision(readSingle(await client.requestDocument(`${root(school)}/${part(advertisement)}/revisions/${part(revision)}`, 'GET', undefined, signal), revisionKind)),
    list: (filters: AdvertisementFilters = {}, signal?: Cancellation) => {
      const query: [string, string][] = [['page[number]', String(filters.page ?? 1)]]
      for (const key of ['school_id', 'status', 'created_from', 'created_to'] as const) if (filters[key]) query.push([key, filters[key]])
      return collection(`/api/v1/platform/message-advertisement-revisions?${query.map(([key, value]) => `${part(key)}=${part(value)}`).join('&')}`, signal)
    },
    decide: (revision: AdvertisementRevision, decision: 'approve' | 'reject', reason: string) => write(`/api/v1/platform/message-advertisement-revisions/${part(revision.id)}/decisions`, 'POST', {
      expected_config_version: revision.config_version, expected_revision_version: revision.version,
      review_digest: revision.review_digest, decision, reason,
    }),
    removePlatform: (config: MessageAdvertisement, reason: string) => write(`/api/v1/platform/message-advertisements/${part(config.id)}/removals`, 'POST', { expected_config_version: config.version, reason }),
  }
}
