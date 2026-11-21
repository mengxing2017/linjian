import type { Cancellation, Client } from '../../http/client'
import { statisticsQuery } from './query'
import type { ActivityInput, SchoolRow, StatisticsQuery, StatisticsReport } from './types'

export function createStatisticsClient(client: Client) {
  const root = (school?: string) =>
    school
      ? `/api/v1/schools/${encodeURIComponent(school)}/statistics`
      : '/api/v1/platform/statistics'
  return {
    report: (school: string | undefined, values: StatisticsQuery, signal?: Cancellation) =>
      client.requestResource<StatisticsReport>(
        `${root(school)}?${statisticsQuery({ ...values })}`,
        'GET',
        undefined,
        signal,
      ),
    schools: (
      snapshot: string,
      values: { page?: number; size?: number; sort?: string; descending?: boolean } = {},
      signal?: Cancellation,
    ) =>
      client.requestCollection<SchoolRow>(
        `${root()}/snapshots/${encodeURIComponent(snapshot)}/schools?${statisticsQuery(values)}`,
        signal,
      ),
    export: (
      school: string | undefined,
      snapshot: string,
      kind: 'periods' | 'schools' = 'periods',
      signal?: Cancellation,
    ) =>
      client.requestBinary(
        `${root(school)}/snapshots/${encodeURIComponent(snapshot)}/export?kind=${kind}`,
        signal,
      ),
    activity: (input: ActivityInput) =>
      client.requestResource<{ id: string; accepted: boolean }>(
        '/api/v1/miniapp/activity-events',
        'POST',
        input,
      ),
  }
}
