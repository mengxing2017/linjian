import type { Cancellation, Client } from '../../http/client'
import type { SchoolRouteSettings, SourceConnectionSettings } from './types'
export function createSourceAccessClient(client: Pick<Client, 'requestResource'>) {
  return {
    connections: () =>
      client.requestResource<SourceConnectionSettings>('/api/v2/source-connections'),
    verify: () =>
      client.requestResource<{ id: string; revision: number }>(
        '/api/v2/source-connections',
        'POST',
        {},
      ),
    bindings: (school: string, signal?: Cancellation) =>
      client.requestResource<SchoolRouteSettings>(
        `/api/v2/schools/${encodeURIComponent(school)}/source-bindings`,
        'GET',
        undefined,
        signal,
      ),
    bind: (school: string, v: { route_id: string; active: boolean }, revision: number) =>
      client.requestResource<{ id: string; revision: number }>(
        `/api/v2/schools/${encodeURIComponent(school)}/source-bindings`,
        'POST',
        v,
        undefined,
        { 'If-Match': `"${revision}"` },
      ),
  }
}
