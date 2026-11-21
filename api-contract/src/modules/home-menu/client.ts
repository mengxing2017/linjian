import type { Client } from '../../http/client'
import { schoolPath } from '../schools/path'
import { homeMenuQuery } from './query'
import type {
  HomeMenu,
  HomeMenuImport,
  HomeMenuQuery,
  HomeMenuSearch,
  HomeMenuSettings,
  HomeMenuTag,
  TagInput,
} from './types'

export function createHomeMenuClient(client: Client) {
  const path = (s: string, suffix: string) => `${schoolPath(s)}/${suffix}`
  return {
    search: (s: string, q: HomeMenuQuery) =>
      client.requestResource<HomeMenuSearch>(
        path(s, 'home-menu-tags/search') + '?' + homeMenuQuery(q),
      ),
    export: (s: string, q: HomeMenuQuery) =>
      client.requestBinary(path(s, 'home-menu-tags/export') + '?' + homeMenuQuery(q)),
    commit: (s: string, id: string, digest: string) =>
      client.requestResource<HomeMenuImport>(
        path(s, `home-menu-imports/${encodeURIComponent(id)}/commit`),
        'POST',
        { digest },
      ),
    public: (s: string) => client.requestResource<HomeMenu>(path(s, 'home-menu')),
    list: (s: string) => client.requestResource<HomeMenu>(path(s, 'home-menu-tags')),
    save: (s: string, input: TagInput, id?: string) =>
      client.requestResource<HomeMenuTag>(
        path(s, id ? `home-menu-tags/${encodeURIComponent(id)}` : 'home-menu-tags'),
        id ? 'PUT' : 'POST',
        input,
      ),
    remove: (s: string, id: string, revision: number) =>
      client.requestResource<null>(path(s, `home-menu-tags/${encodeURIComponent(id)}`), 'DELETE', {
        revision,
      }),
    order: (s: string, ids: string[], catalog_revision: number) =>
      client.requestResource<null>(path(s, 'home-menu-tags/order'), 'POST', {
        ids,
        catalog_revision,
      }),
    getSettings: (s: string) =>
      client.requestResource<HomeMenuSettings>(path(s, 'home-menu-settings')),
    settings: (
      s: string,
      settings: Pick<HomeMenuSettings, 'latest_label' | 'latest_visible' | 'revision'>,
    ) => client.requestResource<HomeMenuSettings>(path(s, 'home-menu-settings'), 'PUT', settings),
  }
}
