import { createSchoolsClient } from '@linjian/api-contract/schools'
import { client } from '../../identity/api'
export type { School } from '@linjian/api-contract/schools'
export const schoolsApi = createSchoolsClient(client)
