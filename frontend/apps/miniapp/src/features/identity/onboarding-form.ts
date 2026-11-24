import { credentials } from '../../app/credentials'
import { identityApi } from './api'
import { legalDocuments } from '../legal-documents/api'
import { loadSchools } from '../school-context/context'
import { acceptLogin, startLogin } from './session'
import { resumeReturnTarget } from './resume-target'
import { createOnboarding } from './use-onboarding'
function requestKey() {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, character => {
    const value = Math.floor(Math.random() * 16)
    return (character === 'x' ? value : (value & 3) | 8).toString(16)
  })
}
export const form = createOnboarding({
  identity: identityApi,
  documents: legalDocuments,
  start: startLogin,
  requestKey,
  ready: async value => {
    acceptLogin(value)
    if (!(await loadSchools())) throw new Error('学校暂不可用，请重试')
    if (!(await resumeReturnTarget())) await uni.reLaunch({ url: '/pages/index/index' })
  },
})
credentials.subscribe(() => form.dispose())
