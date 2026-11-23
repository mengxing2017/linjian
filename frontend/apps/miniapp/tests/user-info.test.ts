import { expect, it, vi } from 'vitest'
import { createUserInfo } from '../src/features/community/use-user-info'
import type { MiniappUser } from '../src/features/miniapp-users/api'

function user(overrides: Partial<MiniappUser> = {}): MiniappUser {
  return { id: 'user', nickname: '用户123456789', revision: 0, avatar_asset_id: 'old', avatar_url: '/old', account_kind: 'ordinary', mobile: null, school_id: null, school_name: null, post_count: 0, created_at: '2026-11-21T00:00:00Z', status: 'normal', ...overrides }
}
function deferred<Value>() {
  let resolve!: (value: Value) => void
  const promise = new Promise<Value>((finish) => { resolve = finish })
  return { promise, resolve }
}
async function setup() {
  const api = { profile: vi.fn().mockResolvedValue(user()), saveProfile: vi.fn().mockResolvedValue(user({ revision: 1 })), registerMobile: vi.fn().mockResolvedValue(user({ revision: 1, mobile: '13800138000' })) }
  const env = { api, restore: vi.fn().mockResolvedValue(true), refreshUser: vi.fn().mockResolvedValue(undefined), upload: vi.fn().mockResolvedValue({ id: 'new', url: '/new' }), key: vi.fn().mockReturnValue('scope'), failure: (cause: unknown) => cause instanceof Error ? cause.message : '失败', confirm: vi.fn().mockResolvedValue(true), notify: vi.fn() }
  const info = createUserInfo(env)
  await info.show()
  return { info, api, env }
}

it('uploads personal images as candidates and saves them only on explicit submission', async () => {
  const { info, api, env } = await setup()
  info.name.value = '我的昵称'
  await info.choose('/personal-photo.png')
  expect(env.upload).toHaveBeenCalledWith('/personal-photo.png')
  expect(info.avatar.value).toBe('/new')
  expect(info.uploadResult.value).toBe('头像已上传，保存后生效')
  expect(api.saveProfile).not.toHaveBeenCalled()
  await info.save()
  expect(api.saveProfile).toHaveBeenCalledWith('我的昵称', 'new', 0)
})

it('retains the previous draft on upload failure and ignores cancellation', async () => {
  const { info, env } = await setup()
  info.name.value = '草稿'
  await info.choose('')
  expect(env.upload).not.toHaveBeenCalled()
  env.upload.mockRejectedValue(new Error('上传失败'))
  await info.choose('/photo')
  expect(info.name.value).toBe('草稿')
  expect(info.asset.value).toBe('old')
  expect(info.error.value).toBe('上传失败')
  expect(info.uploadResult.value).toBe('')
  expect(env.notify).not.toHaveBeenCalled()
})

it('blocks saves while uploads run and snapshots the submitted draft', async () => {
  const { info, api, env } = await setup()
  const upload = deferred<{ id: string; url: string }>()
  env.upload.mockReturnValue(upload.promise)
  const pending = info.choose('/photo')
  await info.save()
  expect(api.saveProfile).not.toHaveBeenCalled()
  upload.resolve({ id: 'candidate', url: '/candidate' })
  await pending
  const saving = deferred<MiniappUser>()
  api.saveProfile.mockReturnValue(saving.promise)
  const first = info.save()
  await info.save()
  expect(api.saveProfile).toHaveBeenCalledOnce()
  saving.resolve(user({ nickname: '已规范化', revision: 2, avatar_asset_id: 'candidate' }))
  await first
  expect(info.name.value).toBe('已规范化')
  expect(info.profile.value?.revision).toBe(2)
})

it('preserves both nickname and avatar on conflicts, and requires explicit reload', async () => {
  const { info, api, env } = await setup()
  info.name.value = '草稿'
  await info.choose('/photo')
  api.saveProfile.mockRejectedValue(Object.assign(new Error('冲突'), { status: 409 }))
  await info.save()
  expect(info.conflict.value).toBe(true)
  expect(info.name.value).toBe('草稿')
  expect(info.asset.value).toBe('new')
  await info.save()
  expect(api.saveProfile).toHaveBeenCalledOnce()
  env.confirm.mockResolvedValue(false)
  await info.reload()
  expect(info.name.value).toBe('草稿')
  env.confirm.mockResolvedValue(true)
  api.profile.mockResolvedValue(user({ nickname: '另一端', revision: 4 }))
  await info.reload()
  expect(info.name.value).toBe('另一端')
  expect(info.conflict.value).toBe(false)
})

it('applies successful writes before refreshing and never repeats a saved write', async () => {
  const { info, api, env } = await setup()
  api.saveProfile.mockResolvedValue(user({ nickname: '新昵称', revision: 1 }))
  env.refreshUser.mockRejectedValueOnce(new Error('网络故障')).mockResolvedValue(undefined)
  await info.save()
  expect(info.profile.value?.revision).toBe(1)
  expect(info.name.value).toBe('新昵称')
  expect(info.refreshWarning.value).toContain('资料已保存')
  await info.retryRefresh()
  expect(info.refreshWarning.value).toBe('')
  expect(api.saveProfile).toHaveBeenCalledOnce()
})

it.each([0, 500, 503])('requires confirmation by reading after ambiguous status %s', async (status) => {
  const { info, api } = await setup()
  info.name.value = '草稿'
  api.saveProfile.mockRejectedValue(Object.assign(new Error('失败'), { status }))
  await info.save()
  expect(info.writeUncertain.value).toBe(true)
  expect(info.name.value).toBe('草稿')
  await info.save()
  expect(api.saveProfile).toHaveBeenCalledOnce()
  await info.reload()
  expect(info.writeUncertain.value).toBe(false)
})

it('does not clear the avatar when only changing nickname and counts Unicode scalars', async () => {
  const { info, api } = await setup()
  info.name.value = '😀'.repeat(100)
  expect(info.length.value).toBe(100)
  expect(info.canSave.value).toBe(true)
  await info.save()
  expect(api.saveProfile).toHaveBeenCalledWith('😀'.repeat(100), 'old', 0)
  info.name.value = '😀'.repeat(101)
  expect(info.canSave.value).toBe(false)
})

it('updates revision after mobile registration without destroying profile drafts', async () => {
  const { info, api } = await setup()
  info.name.value = '未保存'
  info.asset.value = 'candidate'
  info.mobile.value = '13800138000'
  await info.registerMobile()
  expect(info.name.value).toBe('未保存')
  expect(info.asset.value).toBe('candidate')
  await info.save()
  expect(api.saveProfile).toHaveBeenCalledWith('未保存', 'candidate', 1)
})

it.each(['hide', 'dispose', 'scope'])('rejects late results after %s', async (kind) => {
  const { info, api, env } = await setup()
  const pending = deferred<MiniappUser>()
  api.saveProfile.mockReturnValue(pending.promise)
  const saving = info.save()
  if (kind === 'hide') info.hide()
  else if (kind === 'dispose') info.dispose()
  else { env.key.mockReturnValue('other'); info.reset() }
  pending.resolve(user({ nickname: '过期结果', revision: 9 }))
  await saving
  expect(info.name.value).not.toBe('过期结果')
  expect(env.notify).not.toHaveBeenCalled()
  expect(env.refreshUser).not.toHaveBeenCalled()
  expect(info.busy.value).toBe(false)
})

it.each(['hide', 'dispose', 'scope'])('does not announce stale avatar uploads after %s', async kind => {
  const { info, env } = await setup()
  const pending = deferred<{ id: string; url: string }>()
  env.upload.mockReturnValueOnce(pending.promise)
  const uploading = info.choose('/photo')
  if (kind === 'hide') info.hide()
  else if (kind === 'dispose') info.dispose()
  else { env.key.mockReturnValue('other'); info.reset() }
  pending.resolve({ id: 'late', url: '/late' })
  await uploading
  expect(info.uploadResult.value).toBe('')
  expect(info.asset.value).not.toBe('late')
  expect(env.notify).not.toHaveBeenCalled()
})
