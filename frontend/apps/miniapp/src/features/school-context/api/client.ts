import { createSchoolsClient } from '@linjian/api-contract/schools'
import { client } from '../../../app/api'
export const schoolsApi = createSchoolsClient(client)
