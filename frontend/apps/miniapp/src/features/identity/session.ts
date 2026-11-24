import { reactive } from 'vue'
import { clearReturnTarget } from './return-target'
import { credentials } from '../../app/credentials'
import { identityApi, type User, type OnboardingResult, type OnboardingReady } from './api'
export const session = reactive({ user: null as User | null, ready: false, busy: false, error: '' })
let restoring: Promise<void> | undefined
let loggingIn: Promise<OnboardingResult> | undefined
credentials.subscribe(() => {
  session.user = null
  session.ready = false
})
export async function restoreSession() {
  if (session.ready) return
  if (!restoring)
    restoring = (async () => {
      if (!credentials.token) {
        session.ready = true
        return
      }
      const revision = credentials.revision
      try {
        const value = await identityApi.session()
        if (revision === credentials.revision) session.user = value.user
      } catch (error) {
        session.error = error instanceof Error ? error.message : '恢复登录失败'
        throw error
      } finally {
        session.ready = !!session.user || !credentials.token
      }
    })().finally(() => {
      restoring = undefined
    })
  return restoring
}
export function startLogin() {
  if (!loggingIn)
    loggingIn = (async () => {
      session.busy = true
      session.error = ''
      const revision = credentials.revision
      try {
        const result = await uni.login({ provider: 'weixin' })
        if (revision !== credentials.revision) throw new Error('登录操作已取消')
        if (!result.code) throw new Error('微信登录未返回有效凭证，请重试')
        const value = await identityApi.startOnboarding(result.code)
        if (revision !== credentials.revision) throw new Error('登录操作已取消')
        return value
      } catch (error) {
        session.error = error instanceof Error ? error.message : '登录失败'
        throw error
      } finally {
        session.busy = false
      }
    })().finally(() => {
      loggingIn = undefined
    })
  return loggingIn
}
export function acceptLogin(value: OnboardingReady) {
  if (value.state !== 'ready' || !value.token || !value.user.bound_school_id) throw new Error('入驻尚未完成')
  credentials.set(value.token)
  session.user = value.user
  session.ready = true
}
export async function login() {
  await uni.navigateTo({ url: '/pages/user/login' })
}
export async function refreshUser(current: () => boolean = () => true) {
  const revision = credentials.revision
  const value = await identityApi.session()
  if (revision === credentials.revision && current()) session.user = value.user
}
export async function logout() {
  clearReturnTarget()

  try {
    await identityApi.logout()
  } finally {
    try {
      const prefix = `linjian:miniapp:group-qrcode:${session.user?.id ?? ''}:`
      for (const key of uni.getStorageInfoSync().keys) if (key.startsWith(prefix)) uni.removeStorageSync(key)
    } catch { /* Storage cleanup must not prevent local logout. */ }
    credentials.set('')
    session.ready = true
  }
}
export function requireLogin() {
  if (!session.user) {
    uni.navigateTo({ url: '/pages/user/login' })
    return false
  }
  return true
}
