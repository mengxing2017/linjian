import { createStatisticsClient } from '@linjian/api-contract/statistics'
import { client } from '../../identity/api'
export const statisticsApi = createStatisticsClient(client)
export type { ActivityInput } from '@linjian/api-contract/statistics'
