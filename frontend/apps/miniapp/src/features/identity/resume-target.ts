import { loadSchools } from '../school-context/context'
import { restoreSession, session } from './session'
import { readReturnTarget, clearReturnTarget, targetPath } from './return-target'
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
  try {
    if (!(await loadSchools(target.school))) return false
    await uni.reLaunch({ url: targetPath(target) })
    clearReturnTarget()
    return true
  } catch (error) {
    clearReturnTarget()
    throw error
  }
}
