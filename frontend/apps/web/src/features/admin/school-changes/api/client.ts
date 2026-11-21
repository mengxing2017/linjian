import { createSchoolChangesClient } from '@linjian/api-contract/school-changes'
import { client } from '../../../../app/api'
export const schoolChanges = createSchoolChangesClient(client)
