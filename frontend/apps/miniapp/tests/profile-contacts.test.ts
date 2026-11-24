import { expect, it, vi } from 'vitest'
import { createContactsPage } from '../src/features/community/use-contacts-page'
import { createCustomerContacts } from '../src/features/operations/use-customer-contacts'
import { createUserInfo } from '../src/features/community/use-user-info'
import type { Display } from '../src/features/operations/api'
import type { MiniappUser } from '../src/features/miniapp-users/api'

function deferred<Value>() {
  let resolve!: (value: Value) => void
  let reject!: (cause: unknown) => void
  const promise = new Promise<Value>((finish, fail) => { resolve = finish; reject = fail })
  return { promise, resolve, reject }
}
function setup() {
  const contacts = { prepare: vi.fn(), show: vi.fn().mockResolvedValue(undefined), hide: vi.fn() }
  const env = {
    contacts: () => contacts, restore: vi.fn().mockResolvedValue(undefined), loggedIn: vi.fn().mockReturnValue(true),
    loadSchools: vi.fn().mockResolvedValue(true), school: vi.fn().mockReturnValue('school'), revision: vi.fn().mockReturnValue(1),
    key: vi.fn().mockReturnValue('scope'), profile: vi.fn().mockResolvedValue(undefined),
    failure: vi.fn((cause: unknown) => cause instanceof Error ? cause.message : '失败'),
  }
  return { env, contacts, page: createContactsPage(env) }
}

it('loads contacts even if profile fails and does not reload contacts on profile retry', async () => {
  const { env, contacts, page } = setup()
  env.profile.mockRejectedValueOnce(new Error('资料失败'))
  await page.show()
  expect(page.error.value).toBe('资料失败')
  expect(contacts.show).toHaveBeenCalledWith({ state: 'ready', school: 'school' })
  await page.reloadProfile()
  expect(env.profile).toHaveBeenCalledTimes(2)
  expect(contacts.show).toHaveBeenCalledOnce()
  expect(env.loadSchools).toHaveBeenCalledOnce()
})

it('loads profile despite school failure and contact-only preparation retry does not read profile', async () => {
  const { env, contacts, page } = setup()
  env.loadSchools.mockRejectedValueOnce(new Error('学校失败'))
  await page.show()
  expect(env.profile).toHaveBeenCalledOnce()
  expect(contacts.show).toHaveBeenCalledWith({ state: 'error', cause: expect.any(Error) })
  await page.retryPrepare()
  expect(env.profile).toHaveBeenCalledOnce()
  expect(contacts.show).toHaveBeenLastCalledWith({ state: 'ready', school: 'school' })
})

it.each(['anonymous', 'no-school', 'restore-error'])('explains %s without protected contact requests', async mode => {
  const { env, contacts, page } = setup()
  if (mode === 'anonymous') env.loggedIn.mockReturnValue(false)
  if (mode === 'no-school') env.loadSchools.mockResolvedValue(false)
  if (mode === 'restore-error') env.restore.mockRejectedValueOnce(new Error('恢复失败'))
  await page.show()
  expect(contacts.show).toHaveBeenLastCalledWith(expect.objectContaining({ state: mode === 'anonymous' ? 'login-required' : mode === 'no-school' ? 'school-required' : 'error' }))
  if (mode === 'anonymous') expect(env.profile).not.toHaveBeenCalled()
  if (mode === 'restore-error') {
    await page.reloadProfile()
    expect(contacts.show).toHaveBeenLastCalledWith({ state: 'ready', school: 'school' })
  }
  expect(page.busy.value).toBe(false)
})

it.each(['restore', 'schools'])('rejects late %s preparation after hide, disposal or credentials change', async stage => {
  for (const action of ['hide', 'dispose', 'credentials']) {
    const { env, contacts, page } = setup()
    const first = deferred<boolean>()
    if (stage === 'restore') env.restore.mockImplementationOnce(async () => { await first.promise })
    else env.loadSchools.mockReturnValueOnce(first.promise)
    const pending = page.show()
    if (stage === 'schools') await Promise.resolve()
    if (action === 'hide') page.hide()
    else if (action === 'dispose') page.dispose()
    else { env.revision.mockReturnValue(2); page.resetProfile() }
    first.resolve(true)
    await pending
    expect(contacts.show).not.toHaveBeenCalled()
    expect(env.profile).not.toHaveBeenCalled()
    expect(page.busy.value).toBe(false)
  }
})

it('prepares identity again after credential invalidation instead of reusing a previous session', async () => {
  const { env, page, contacts } = setup()
  await page.show()
  env.revision.mockReturnValue(2)
  env.loggedIn.mockReturnValue(false)
  page.resetProfile()
  await page.reloadProfile()
  expect(env.restore).toHaveBeenCalledTimes(2)
  expect(contacts.show).toHaveBeenLastCalledWith({ state: 'login-required' })
})

it('accepts expected school selection and uses the final school context', async () => {
  const { env, contacts, page } = setup()
  env.loadSchools.mockImplementationOnce(async () => { env.key.mockReturnValue('new-scope'); env.school.mockReturnValue('new-school'); page.resetProfile(); return true })
  await page.show()
  expect(contacts.show).toHaveBeenCalledWith({ state: 'ready', school: 'new-school' })
  expect(env.profile).toHaveBeenCalledOnce()
})

it('prepares contacts again while profile is pending without invalidating the profile result', async () => {
  const { env, contacts, page } = setup()
  const result = deferred<void>()
  let active = () => false
  env.profile.mockImplementationOnce(async current => { active = current; await result.promise })
  const pending = page.show()
  await vi.waitFor(() => expect(env.profile).toHaveBeenCalledOnce())
  await page.retryPrepare()
  expect(contacts.show).toHaveBeenCalledTimes(2)
  expect(active()).toBe(true)
  result.resolve()
  await pending
  expect(page.busy.value).toBe(false)
})

it('ignores older profile finally and results when a page is hidden and reentered', async () => {
  const { env, page } = setup()
  const old = deferred<void>(), next = deferred<void>()
  let oldCurrent = () => true
  env.profile.mockImplementationOnce(async current => { oldCurrent = current; await old.promise }).mockReturnValueOnce(next.promise)
  const first = page.show()
  await vi.waitFor(() => expect(env.profile).toHaveBeenCalledOnce())
  page.hide()
  const second = page.show()
  await vi.waitFor(() => expect(env.profile).toHaveBeenCalledTimes(2))
  old.resolve()
  await first
  expect(oldCurrent()).toBe(false)
  expect(page.busy.value).toBe(true)
  next.resolve()
  await second
  expect(page.busy.value).toBe(false)
})

it('does not clear user-info drafts or invoke writes on contact loading, retry, preview and close', async () => {
  const { env } = setup()
  const user = { id: 'user', revision: 1, nickname: '原昵称', avatar_asset_id: null } as MiniappUser
  const api = { profile: vi.fn().mockResolvedValue(user), saveProfile: vi.fn(), registerMobile: vi.fn() }
  const info = createUserInfo({ api, restore: async () => true, refreshUser: vi.fn(), upload: vi.fn().mockResolvedValue({ id: 'draft', url: '/draft' }), key: env.key, failure: env.failure, confirm: vi.fn(), notify: vi.fn() })
  const display = vi.fn().mockResolvedValue({ customer: { enabled: true, service: '/service', note: '' } } as Display)
  const contacts = createCustomerContacts({ display, image: vi.fn().mockResolvedValue('/image'), key: env.key, failure: env.failure, notify: vi.fn(), preview: vi.fn().mockResolvedValue(undefined) })
  const page = createContactsPage({ ...env, contacts: () => contacts, profile: () => info.show() })
  await page.show()
  info.name.value = '未保存昵称'
  await info.choose('/photo')
  display.mockRejectedValueOnce(new Error('配置故障'))
  await contacts.show({ state: 'ready', school: 'school' })
  await contacts.retry()
  await contacts.open('service')
  await contacts.preview()
  contacts.close()
  await page.retryPrepare()
  expect(info.name.value).toBe('未保存昵称')
  expect(info.asset.value).toBe('draft')
  expect(api.profile).toHaveBeenCalledOnce()
  expect(api.saveProfile).not.toHaveBeenCalled()
  expect(api.registerMobile).not.toHaveBeenCalled()
})
