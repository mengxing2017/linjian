import { createSchoolsClient } from '@linjian/api-contract/schools'
import { client } from '../../../app/api'
const schools = createSchoolsClient(client)
export const schoolsApi = {
  detail: schools.detail,
  directory: schools.directory,
  onboardingRecommendations: schools.onboardingRecommendations,
  branding: schools.branding,
}
