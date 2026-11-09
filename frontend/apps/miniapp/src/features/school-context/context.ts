import { shallowReactive } from 'vue'
import { credentials } from '../../app/credentials'
import { cancelRequests } from '../../app/transport'
import { refreshUser, restoreSession, session } from '../identity/session'
import { schoolsApi, type School } from './api'
export const schoolContext = shallowReactive({ schools: [] as School[], id: '', generation: 0 })
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
credentials.subscribe(() => {
  clearScope()
  schoolContext.schools = []
  schoolContext.id = ''
})
export const scopeKey = () =>
  `${credentials.revision}:${schoolContext.generation}:${schoolContext.id}`
export function selectSchool(id: string) {
  if (!schoolContext.schools.some((s) => s.id === id)) throw new Error('没有该学校的访问权限')
  if (id !== schoolContext.id) {
    if (schoolContext.id) clearScope()
    schoolContext.id = id
  }
}
export async function loadSchools(preferred?: string) {
  await restoreSession()
  if (!session.user) return false
  const revision = credentials.revision
  const schools = await schoolsApi.list()
  if (revision !== credentials.revision) return false
  const old = schoolContext.schools.find((s) => s.id === schoolContext.id)
  const next = schools.find((s) => s.id === schoolContext.id)
  if (old && (!next || old.revision !== next.revision)) clearScope()
  schoolContext.schools = schools.filter((s) => s.status === 'normal')
  const id = preferred || schoolContext.id || session.user.bound_school_id || schools[0]?.id
  if (id && schools.some((s) => s.id === id)) selectSchool(id)
  else if (preferred) {
    clearScope()
    schoolContext.id = ''
    throw new Error('没有该学校的访问权限')
  } else {
    if (schoolContext.id) clearScope()
    schoolContext.id = ''
    if (schools[0]) selectSchool(schools[0].id)
  }
  return !!schoolContext.id
}
export async function bindSchool(id: string) {
  await schoolsApi.bind(id)
  await refreshUser()
  await loadSchools(id)
}
export async function requireSchool(preferred?: string) {
  if (!(await loadSchools(preferred))) {
    if (session.user?.bound_school_id) throw new Error('学校暂不可用，请联系管理员')
    uni.navigateTo({ url: session.user ? '/pages/plugin/index' : '/pages/user/login' })
    return false
  }
  return true
}

export function invalidateSchool() {
  clearScope()
  schoolContext.schools = []
  schoolContext.id = ''
}
