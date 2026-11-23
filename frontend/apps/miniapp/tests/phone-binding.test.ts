import { expect, it, vi } from 'vitest'
import { createPhoneBinding } from '../src/features/miniapp-users/phone-binding'
import type { PhoneBinding } from '../src/features/miniapp-users/api'

const unverified: PhoneBinding = { id: 'account', status: 'unverified', masked_phone: null, verified_at: null, revision: 0 }
const verified: PhoneBinding = { ...unverified, status: 'verified', masked_phone: '*******8000', verified_at: '2026-11-23T00:00:00Z', revision: 1 }
function setup() {
  let scope = 'account:school'
  const api = { phoneBinding: vi.fn().mockResolvedValue(unverified), bindPhone: vi.fn().mockResolvedValue(verified), revokePhone: vi.fn().mockResolvedValue({ ...unverified, status: 'revoked', revision: 2 }) }
  const notify = vi.fn(), confirm = vi.fn().mockResolvedValue(true)
  const phone = createPhoneBinding({ api, key: () => scope, uuid: () => 'new-request', confirm, notify, failure: cause => cause instanceof Error ? cause.message : '验证失败' })
  return { phone, api, notify, confirm, changeScope() { scope = 'other-account'; phone.reset() } }
}

it('does not request phone information on construction or browsing and cancellation is non-destructive', async () => {
  const { phone, api } = setup()
  expect(api.phoneBinding).not.toHaveBeenCalled()
  expect(phone.visible.value).toBe(false)
  expect(await phone.ensure()).toBe(false)
  expect(phone.visible.value).toBe(true)
  await phone.authorize({ detail: { errMsg: 'getPhoneNumber:fail user deny' } })
  expect(api.bindPhone).not.toHaveBeenCalled()
  phone.close()
  expect(phone.visible.value).toBe(false)
})

it('sends only a one-use code with expected revision and never replays the original content action', async () => {
  const { phone, api, notify } = setup()
  await phone.open()
  await phone.authorize({ detail: { code: 'once-code' } })
  expect(api.bindPhone).toHaveBeenCalledExactlyOnceWith({ code: 'once-code', request_id: 'new-request', revision: 0, notice_version: 'linjian-phone-v1' })
  expect(phone.visible.value).toBe(false)
  expect(phone.binding.value).toEqual(verified)
  expect(notify).toHaveBeenCalledWith('验证完成，请确认后再提交')
  expect(Object.keys(api)).toEqual(['phoneBinding', 'bindPhone', 'revokePhone'])
})

it('checks the server rather than using a stale verified display and preserves legitimate reading after revocation', async () => {
  const { phone, api } = setup()
  api.phoneBinding.mockResolvedValueOnce(verified)
  expect(await phone.ensure()).toBe(true)
  expect(phone.visible.value).toBe(false)
  await phone.revoke()
  expect(api.revokePhone).toHaveBeenCalledWith(1)
  expect(phone.binding.value?.status).toBe('revoked')
  expect(await phone.ensure()).toBe(false)
  expect(phone.visible.value).toBe(true)
})

it('confirms uncertain writes by reading status without automatically consuming a code again', async () => {
  const { phone, api } = setup()
  await phone.open()
  api.bindPhone.mockRejectedValue(new Error('响应丢失'))
  api.phoneBinding.mockResolvedValue(verified)
  await phone.authorize({ detail: { code: 'once-code' } })
  expect(api.bindPhone).toHaveBeenCalledOnce()
  expect(phone.binding.value?.status).toBe('verified')
  expect(phone.visible.value).toBe(false)
})

it('ignores a delayed binding result after account or school changes', async () => {
  const { phone, api, changeScope, notify } = setup()
  await phone.open()
  let finish!: (value: PhoneBinding) => void
  api.bindPhone.mockImplementation(() => new Promise<PhoneBinding>(resolve => { finish = resolve }))
  const pending = phone.authorize({ detail: { code: 'once-code' } })
  changeScope()
  finish(verified)
  await pending
  expect(phone.binding.value).toBeUndefined()
  expect(phone.visible.value).toBe(false)
  expect(notify).not.toHaveBeenCalled()
})

it('ignores late callbacks when the authorization panel has been closed', async () => {
  const { phone, api } = setup()
  await phone.open()
  phone.close()
  await phone.authorize({ detail: { code: 'late-code' } })
  expect(api.bindPhone).not.toHaveBeenCalled()
})
