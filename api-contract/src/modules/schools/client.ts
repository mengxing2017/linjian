import type { Cancellation, Client } from '../../http/client'
import { schoolPath } from './path'
import { schoolQuery } from './query'
import type { School, SchoolDisplay, SchoolFilters, SchoolInput } from './types'

export function createSchoolsClient(
  client: Pick<Client, 'requestResource' | 'requestCollection' | 'requestBinary'>,
) {
  return {
    list: () => client.requestResource<SchoolDisplay[]>('/api/v1/schools'),
    platform: (filters: SchoolFilters = {}, signal?: Cancellation) =>
      client.requestCollection<School>('/api/v1/platform/schools' + schoolQuery(filters), signal),
    directory: (filters: SchoolFilters = {}, signal?: Cancellation) =>
      client.requestCollection<SchoolDisplay>(
        '/api/v1/school-directory' + schoolQuery(filters),
        signal,
      ),
    detail: (school: string, signal?: Cancellation) =>
      client.requestResource<School>(schoolPath(school), 'GET', undefined, signal),
    create: (body: SchoolInput) => client.requestResource<School>('/api/v1/schools', 'POST', body),
    update: (school: string, body: SchoolInput) =>
      client.requestResource<School>(schoolPath(school), 'PUT', body),
    bind: (school: string) => client.requestResource<null>(`${schoolPath(school)}/bind`, 'POST'),
    branding: (school: string, slot: 'logo' | 'header') =>
      client.requestBinary(
        `/api/v1/school-directory/${encodeURIComponent(school)}/branding/${slot}`,
      ),
    artwork: (school: string, id: string) =>
      client.requestBinary(
        `${schoolPath(school)}/resource-artwork/${encodeURIComponent(id)}?context=school.profile.read`,
      ),
  }
}
