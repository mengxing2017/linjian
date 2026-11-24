import { computed, ref, shallowRef } from 'vue'
import type { Display } from './api'

export const contactEntries = [
  { kind: 'service', title: '平台客服' },
  { kind: 'join', title: '我要加盟' },
  { kind: 'business', title: '商务合作' },
] as const
export type ContactKind = typeof contactEntries[number]['kind']
export type ContactContext =
  | { state: 'login-required' | 'school-required' }
  | { state: 'error'; cause: unknown }
  | { state: 'ready'; school: string }
type ContactState = 'idle' | 'preparing' | 'login-required' | 'school-required' | 'loading' | 'disabled' | 'ready' | 'error'

interface Environment {
  display: (school: string) => Promise<Display>
  image: (school: string, source: string) => Promise<string>
  key: () => string
  failure: (cause: unknown) => string
  notify: (message: string) => void
  preview: (path: string) => Promise<unknown>
}

export function createCustomerContacts(env: Environment) {
  const state = ref<ContactState>('idle')
  const config = shallowRef<Display['customer']>()
  const error = ref('')
  const failureStage = ref<'prepare' | 'config'>('prepare')
  const selected = ref<ContactKind>()
  const imageState = ref<'idle' | 'loading' | 'ready' | 'error'>('idle')
  const path = ref('')
  const imageError = ref('')
  const title = computed(() => contactEntries.find(entry => entry.kind === selected.value)?.title ?? '')
  let visible = false
  let disposed = false
  let generation = 0
  let selection = 0
  let school = ''
  let key = ''

  function close() {
    selection++
    selected.value = undefined
    imageState.value = 'idle'
    path.value = ''
    imageError.value = ''
  }

  function clear() {
    generation++
    close()
    config.value = undefined
    error.value = ''
    school = ''
    key = ''
  }

  function prepare() {
    if (disposed) return
    visible = true
    clear()
    state.value = 'preparing'
  }

  function current(turn: number, scope: string) {
    return visible && !disposed && generation === turn && scope === env.key()
  }

  async function show(context: ContactContext) {
    if (disposed) return
    visible = true
    clear()
    if (context.state === 'error') {
      const turn = generation
      const scope = env.key()
      const message = env.failure(context.cause)
      if (current(turn, scope)) {
        state.value = 'error'
        failureStage.value = 'prepare'
        error.value = message
      }
      return
    }
    if (context.state !== 'ready') {
      state.value = context.state
      return
    }
    school = context.school
    key = env.key()
    const scope = key
    const turn = generation
    state.value = 'loading'
    failureStage.value = 'config'
    try {
      const result = await env.display(school)
      if (!current(turn, scope)) return
      if (!result.customer || typeof result.customer.enabled !== 'boolean') throw new Error('平台服务配置格式错误')
      if ((result.customer.note !== undefined && typeof result.customer.note !== 'string') || contactEntries.some(entry => {
        const source = result.customer[entry.kind]
        return source !== undefined && source !== null && typeof source !== 'string'
      })) throw new Error('平台服务配置格式错误')
      config.value = result.customer
      state.value = result.customer.enabled ? 'ready' : 'disabled'
    } catch (cause) {
      if (!current(turn, scope)) return
      const message = env.failure(cause)
      if (current(turn, scope)) {
        state.value = 'error'
        error.value = message
      }
    }
  }

  function retry() {
    if (!visible || disposed || state.value !== 'error' || failureStage.value !== 'config') return
    if (key !== env.key()) { reset(); return }
    return show({ state: 'ready', school })
  }

  async function open(kind: ContactKind) {
    if (!visible || disposed || state.value !== 'ready' || key !== env.key()) return
    if (selected.value === kind && imageState.value === 'loading') return
    close()
    const source = config.value?.[kind]
    if (!source) { env.notify('暂未配置联系方式'); return }
    selected.value = kind
    imageState.value = 'loading'
    const turn = generation
    const choice = selection
    const scope = key
    const active = () => current(turn, scope) && choice === selection
    try {
      const result = await env.image(school, source)
      if (!active()) return
      path.value = result
      imageState.value = 'ready'
    } catch (cause) {
      if (!active()) return
      const message = env.failure(cause)
      if (active()) {
        imageState.value = 'error'
        imageError.value = `二维码加载失败：${message}`
      }
    }
  }

  function retryImage() {
    if (selected.value) return open(selected.value)
  }

  async function preview() {
    if (!visible || disposed || key !== env.key() || imageState.value !== 'ready') return
    const turn = generation
    const choice = selection
    const scope = key
    try { await env.preview(path.value) }
    catch { if (current(turn, scope) && choice === selection) env.notify('图片预览失败，请重试') }
  }

  function reset() {
    clear()
    state.value = visible && !disposed ? 'error' : 'idle'
    failureStage.value = 'prepare'
    if (state.value === 'error') error.value = '登录或学校状态已变化，请重新加载'
  }

  function hide() {
    visible = false
    clear()
    state.value = 'idle'
  }

  function dispose() {
    hide()
    disposed = true
  }

  return { state, config, error, failureStage, selected, imageState, path, imageError, title, prepare, show, retry, open, retryImage, close, preview, reset, hide, dispose }
}
