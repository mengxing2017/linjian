import { shallowReactive } from 'vue'
import { credentials } from '../../app/credentials'
import { cancelRequests } from '../../app/transport'
import { restoreSession, session } from '../identity/session'
import { schoolsApi, type School } from './api'
export const schoolContext = shallowReactive({ school: undefined as School | undefined, id: '', generation: 0, loaded: false })
let epoch = 0
let loading: { revision: number; school: string; promise: Promise<boolean> } | undefined
const listeners = new Set<() => void>()
export function onScopeChange(fn: () => void) {
  listeners.add(fn)
  return () => listeners.delete(fn)
}
export function clearScope() {
  cancelRequests()
  schoolContext.generation++
  listeners.forEach((fn) => fn())
}
credentials.subscribe(invalidateSchool)
export const scopeKey = () =>
  `${credentials.revision}:${schoolContext.generation}:${schoolContext.id}`
export async function loadSchool(expected?: string) {
  await restoreSession()
  const id = session.user?.bound_school_id
  if (!id) {
    if (schoolContext.school || schoolContext.id || loading) invalidateSchool()
    return false
  }
  if (expected && expected !== id) throw new Error('只能访问当前绑定学校')
  const revision = credentials.revision
  if (schoolContext.id && schoolContext.id !== id) invalidateSchool()
  if (loading?.revision === revision && loading.school === id) return loading.promise
  const turn = epoch
  schoolContext.loaded = false
  const promise = (async () => {
    const school = await schoolsApi.detail(id)
    if (turn !== epoch || revision !== credentials.revision || id !== session.user?.bound_school_id) return false
    if (school.id !== id) throw new Error('学校资料与绑定学校不一致')
    const next = school.status === 'normal' ? school : undefined
    if (schoolContext.school && (!next || schoolContext.school.revision !== next.revision)) clearScope()
    schoolContext.school = next
    schoolContext.id = next?.id ?? ''
    schoolContext.loaded = true
    return !!next
  })().finally(() => { if (loading?.promise === promise) loading = undefined })
  loading = { revision, school: id, promise }
  return promise
}
export async function requireSchool(expected?: string) {
  const revision = credentials.revision, turn = epoch
  if (!(await loadSchool(expected))) {
    if (revision !== credentials.revision || turn !== epoch) return false
    if (session.user?.bound_school_id) throw new Error('学校暂不可用，请联系管理员')
    uni.navigateTo({ url: '/pages/user/login' })
    return false
  }
  return true
}

export function invalidateSchool() {
  epoch++
  loading = undefined
  schoolContext.school = undefined
  schoolContext.id = ''
  schoolContext.loaded = false
  clearScope()
}
