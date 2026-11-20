import { client } from '@linjian/api-contract/http'
import { createStatisticsClient } from '@linjian/api-contract/statistics'
export const statisticsApi = createStatisticsClient(client)
export type {
  StatisticsView,
  StatisticsQuery,
  StatisticsReport,
  Metrics,
  Bucket,
  Change,
  SchoolRow,
  Period,
} from '@linjian/api-contract/statistics'

import { createSchoolPermissionsClient } from '@linjian/api-contract/schools'
export const statisticsPermissions = createSchoolPermissionsClient(client)
