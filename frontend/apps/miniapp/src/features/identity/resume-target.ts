import { loadSchool } from '../school-context/context'
import { restoreSession, session } from './session'
import { readReturnTarget, clearReturnTarget, targetPath } from './return-target'
import { rememberPublishSource, switchRoot } from '../../app/navigation'
import { credentials } from '../../app/credentials'
let resuming: Promise<boolean> | undefined
export function resumeReturnTarget() {
  if (!resuming)
    resuming = resume().finally(() => {
      resuming = undefined
    })
  return resuming
}
async function resume() {
  const target = readReturnTarget()
  if (!target) return false
  await restoreSession()
  if (!session.user) return false
  if (target.kind === 'publish' && target.account && target.account !== session.user.id) {
    clearReturnTarget()
    return false
  }
  const revision = credentials.revision
  try {
    if (!(await loadSchool(target.kind === 'post-detail' ? target.school : undefined))) return false
    if (revision !== credentials.revision) return false
    if (target.kind === 'publish') {
      await switchRoot(target.origin)
      if (revision !== credentials.revision) return false
      rememberPublishSource(target.origin)
      await uni.navigateTo({ url: targetPath(target) })
    } else await uni.reLaunch({ url: targetPath(target) })
    clearReturnTarget()
    return true
  } catch (error) {
    clearReturnTarget()
    throw error
  }
}
