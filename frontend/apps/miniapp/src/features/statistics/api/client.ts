import { createStatisticsClient } from '@linjian/api-contract/statistics'
import { client } from '../../../app/api'
export const statisticsApi = createStatisticsClient(client)
