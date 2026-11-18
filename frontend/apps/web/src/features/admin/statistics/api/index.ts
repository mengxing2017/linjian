import { client } from '@linjian/api-contract/http'
import { createStatisticsClient } from '@linjian/api-contract/statistics'
export const statisticsApi = createStatisticsClient(client)
export type {
  StatisticsQuery,
  StatisticsReport,
  Metrics,
  Bucket,
  Change,
  SchoolRow,
  Period,
} from '@linjian/api-contract/statistics'
