import { computed, ref, shallowRef } from 'vue'
import type { miniappUserApi, MiniappUser } from '../miniapp-users/api'

interface Environment {
  api: Pick<typeof miniappUserApi, 'profile' | 'saveProfile' | 'registerMobile'>
  restore: () => Promise<boolean>
  refreshUser: (current: () => boolean) => Promise<void>
  upload: (path: string) => Promise<{ id: string; url: string }>
  key: () => string
  failure: (cause: unknown) => string
  confirm: (message: string) => Promise<boolean>
  notify: (message: string) => void
}

function status(cause: unknown) {
  return cause && typeof cause === 'object' && 'status' in cause ? Number(cause.status) : 0
}

export function createUserInfo(env: Environment) {
  const profile = shallowRef<MiniappUser>()
  const name = ref('')
  const mobile = ref('')
  const asset = ref<string | null>(null)
  const avatar = ref<string | null>(null)
  const operation = ref('')
  const error = ref('')
  const conflict = ref(false)
  const refreshWarning = ref('')
  const writeUncertain = ref(false)
  const length = computed(() => Array.from(name.value.trim()).length)
  const busy = computed(() => !!operation.value)
  const canSave = computed(() => !!profile.value && !busy.value && !conflict.value && !writeUncertain.value && length.value > 0 && length.value <= 100)
  let generation = 0
  let visible = false
  let disposed = false

  function currentToken() {
    const turn = ++generation
    const key = env.key()
    return () => !disposed && visible && generation === turn && key === env.key()
  }

  function apply(value: MiniappUser, replaceDraft = true) {
    profile.value = value
    mobile.value = value.mobile ?? ''
    if (replaceDraft) {
      name.value = value.nickname
      asset.value = value.avatar_asset_id
      avatar.value = value.avatar_url
    }
  }

  function fail(cause: unknown) {
    const turn = generation
    const message = env.failure(cause)
    if (turn === generation && visible && !disposed) error.value = message
  }

  async function run(kind: string, action: (current: () => boolean) => Promise<void>) {
    if (busy.value || disposed || !visible) return
    const current = currentToken()
    operation.value = kind
    error.value = ''
    try {
      await action(current)
    } catch (cause) {
      if (current()) fail(cause)
    } finally {
      if (current()) operation.value = ''
    }
  }

  async function read(current: () => boolean) {
    const value = await env.api.profile()
    if (!current()) return
    apply(value)
    conflict.value = false
    writeUncertain.value = false
  }

  function show() {
    visible = true
    return run('loading', async (current) => {
      if (!await env.restore() || !current()) return
      await read(current)
    })
  }

  function hide() {
    visible = false
    generation++
    operation.value = ''
  }

  function reset() {
    generation++
    operation.value = ''
    profile.value = undefined
    name.value = ''
    mobile.value = ''
    asset.value = null
    avatar.value = null
    error.value = ''
    conflict.value = false
    writeUncertain.value = false
    refreshWarning.value = ''
  }

  function choose(path: string) {
    if (!path) return Promise.resolve()
    return run('uploading', async (current) => {
      const value = await env.upload(path)
      if (!current()) return
      asset.value = value.id
      avatar.value = value.url
    })
  }

  async function refresh(current: () => boolean) {
    try {
      await env.refreshUser(current)
      if (current()) refreshWarning.value = ''
    } catch {
      if (current()) refreshWarning.value = '资料已保存，显示信息刷新失败，请重试刷新'
    }
  }

  function save() {
    if (!canSave.value || !profile.value) return Promise.resolve()
    const draft = { nickname: name.value, asset: asset.value, revision: profile.value.revision }
    return run('saving', async (current) => {
      let value: MiniappUser
      try {
        value = await env.api.saveProfile(draft.nickname, draft.asset, draft.revision)
      } catch (cause) {
        if (!current()) return
        if (status(cause) === 409) {
          conflict.value = true
          error.value = '资料已变化，当前输入已保留，请重新加载后编辑'
        } else if (!status(cause) || status(cause) >= 500) {
          writeUncertain.value = true
          error.value = '保存结果待确认，请重新读取资料确认，不要重复提交'
        } else fail(cause)
        return
      }
      if (!current()) return
      apply(value)
      conflict.value = false
      writeUncertain.value = false
      env.notify('已更新')
      await refresh(current)
    })
  }

  function reload() {
    return run('loading', async (current) => {
      if (profile.value && !await env.confirm('重新读取将丢弃当前昵称和头像草稿，是否继续？')) return
      if (current()) await read(current)
    })
  }

  function retryRefresh() {
    return run('refreshing', (current) => refresh(current))
  }

  function registerMobile() {
    if (!profile.value || conflict.value || writeUncertain.value) return Promise.resolve()
    const revision = profile.value.revision
    const value = mobile.value
    return run('mobile', async (current) => {
      try {
        const result = await env.api.registerMobile(value, revision)
        if (!current()) return
        apply(result, false)
        env.notify('手机号已登记')
      } catch (cause) {
        if (!current()) return
        if (status(cause) === 409) conflict.value = true
        if (!status(cause) || status(cause) >= 500) writeUncertain.value = true
        fail(cause)
      }
    })
  }

  return {
    profile, name, mobile, asset, avatar, busy, operation, error, conflict,
    refreshWarning, writeUncertain, length, canSave,
    show, hide, reset, choose, save, reload, retryRefresh, registerMobile,
    dispose() { hide(); disposed = true },
  }
}
