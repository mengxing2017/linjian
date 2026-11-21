import { createSchoolChangesClient } from '@linjian/api-contract/school-changes'
import { client } from '../../../app/api'
export const schoolChangesApi = createSchoolChangesClient(client)
