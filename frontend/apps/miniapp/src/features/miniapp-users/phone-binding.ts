import { ref, shallowRef } from 'vue'
import type { miniappUserApi, PhoneBinding } from './api'

interface Environment {
  api: Pick<typeof miniappUserApi, 'phoneBinding' | 'bindPhone' | 'revokePhone'>
  key: () => string
  uuid: () => string
  confirm: (message: string) => Promise<boolean>
  failure: (cause: unknown) => string
  notify: (message: string) => void
}

export function createPhoneBinding(env: Environment) {
  const binding = shallowRef<PhoneBinding>()
  const visible = ref(false)
  const busy = ref(false)
  const error = ref('')
  let generation = 0
  let disposed = false
  function token() {
    const turn = generation
    const scope = env.key()
    return () => !disposed && turn === generation && scope === env.key()
  }
  function reset() {
    generation++
    binding.value = undefined
    visible.value = false
    busy.value = false
    error.value = ''
  }
  function close() { generation++; visible.value = false; busy.value = false; error.value = '' }
  async function refresh() {
    const current = token()
    const value = await env.api.phoneBinding()
    if (!current()) return false
    binding.value = value
    return true
  }
  async function open() {
    if (busy.value || disposed) return
    visible.value = true
    error.value = ''
    busy.value = true
    const current = token()
    try { await refresh() }
    catch (cause) { if (current()) error.value = env.failure(cause) }
    finally { if (current()) busy.value = false }
  }
  async function ensure() {
    if (busy.value || disposed) return false
    const current = token()
    busy.value = true
    error.value = ''
    try {
      if (!await refresh() || !current()) return false
      if (binding.value?.status === 'verified') return true
      visible.value = true
      return false
    } catch (cause) {
      if (current()) { visible.value = true; error.value = env.failure(cause) }
      return false
    } finally { if (current()) busy.value = false }
  }
  async function authorize(event: { detail: { code?: string; errMsg?: string } }) {
    if (busy.value || !visible.value || disposed) return
    if (!event.detail.code) {
      error.value = '未完成授权，暂不验证仍可继续浏览。需要发布时可再次验证。'
      return
    }
    if (!binding.value) { await open(); return }
    const current = token()
    const revision = binding.value.revision
    busy.value = true
    error.value = ''
    try {
      const value = await env.api.bindPhone({ code: event.detail.code, request_id: env.uuid(), revision, notice_version: 'linjian-phone-v1' })
      if (!current()) return
      binding.value = value
      if (value.status === 'verified') {
        visible.value = false
        env.notify('验证完成，请确认后再提交')
      } else error.value = '绑定状态已变化，请重新读取后验证'
    } catch (cause) {
      if (!current()) return
      error.value = env.failure(cause)
      try {
        await refresh()
        if (current() && binding.value?.status === 'verified' && binding.value.revision > revision) {
          visible.value = false
          error.value = ''
          env.notify('手机号已验证，请确认后再提交')
        }
      } catch { if (current()) error.value = env.failure(cause) }
    } finally { if (current()) busy.value = false }
  }
  async function revoke() {
    if (busy.value || !binding.value || disposed) return
    const current = token()
    if (!await env.confirm('撤回绑定将停止普通用户发布、评论和点赞，仍可登录浏览。确认撤回并删除当前绑定号码？') || !current()) return
    busy.value = true
    error.value = ''
    try {
      const value = await env.api.revokePhone(binding.value.revision)
      if (!current()) return
      binding.value = value
      env.notify('已撤回绑定，仍可继续浏览')
    } catch (cause) { if (current()) error.value = env.failure(cause) }
    finally { if (current()) busy.value = false }
  }
  return { binding, visible, busy, error, refresh, open, ensure, authorize, revoke, close, reset, dispose() { reset(); disposed = true } }
}
