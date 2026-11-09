import { createSchoolsClient } from '@linjian/api-contract/schools'
import { client } from '../../identity/api'
export type { SchoolDisplay as School } from '@linjian/api-contract/schools'
export const schoolsApi = createSchoolsClient(client)

export type { SchoolDisplay } from '@linjian/api-contract/schools'
