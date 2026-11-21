import { createStatisticsClient } from '@linjian/api-contract/statistics'
import { createSchoolPermissionsClient } from '@linjian/api-contract/schools'
import { client } from '../../../../app/api'
export const statisticsApi = createStatisticsClient(client)
export const statisticsPermissions = createSchoolPermissionsClient(client)
