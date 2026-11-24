import { expect, it, vi } from 'vitest'
import { contactEntries, createCustomerContacts } from '../src/features/operations/use-customer-contacts'
import type { Display } from '../src/features/operations/api'

function display(customer: Display['customer'] = { enabled: true, note: '备注来意', service: '/service?revision=1', join: '/join?revision=1', business: null }): Display {
  return { id: 'current', server_time: '', qrcode: null, banners: [], shortcuts: [], categories: [], config_token: '', customer }
}
function deferred<Value>() {
  let resolve!: (value: Value) => void
  let reject!: (cause: unknown) => void
  const promise = new Promise<Value>((finish, fail) => { resolve = finish; reject = fail })
  return { promise, resolve, reject }
}
function setup() {
  const env = {
    display: vi.fn().mockResolvedValue(display()), image: vi.fn().mockResolvedValue('/local-image'),
    key: vi.fn().mockReturnValue('school:1'), failure: vi.fn((cause: unknown) => cause instanceof Error ? cause.message : '失败'),
    notify: vi.fn(), preview: vi.fn().mockResolvedValue(undefined),
  }
  return { env, contacts: createCustomerContacts(env) }
}
const ready = { state: 'ready', school: 'school' } as const

it('orders the three entries and distinguishes preparation, identity, disabled and missing images', async () => {
  const { env, contacts } = setup()
  expect(contactEntries.map(entry => entry.title)).toEqual(['平台客服', '我要加盟', '商务合作'])
  contacts.prepare()
  expect(contacts.state.value).toBe('preparing')
  await contacts.show({ state: 'login-required' })
  expect(contacts.state.value).toBe('login-required')
  await contacts.show({ state: 'school-required' })
  expect(contacts.state.value).toBe('school-required')
  expect(env.display).not.toHaveBeenCalled()
  env.display.mockResolvedValueOnce(display({ enabled: false }))
  await contacts.show(ready)
  expect(contacts.state.value).toBe('disabled')
  await contacts.show(ready)
  await contacts.open('business')
  expect(contacts.state.value).toBe('ready')
  expect(env.notify).toHaveBeenCalledWith('暂未配置联系方式')
  expect(env.image).not.toHaveBeenCalled()
})

it('shows configuration failures, retries only configuration and rejects malformed responses', async () => {
  const { env, contacts } = setup()
  env.display.mockRejectedValueOnce(new Error('网络故障'))
  await contacts.show(ready)
  expect(contacts.state.value).toBe('error')
  expect(contacts.error.value).toBe('网络故障')
  expect(contacts.failureStage.value).toBe('config')
  await contacts.retry()
  expect(contacts.state.value).toBe('ready')
  expect(env.display).toHaveBeenLastCalledWith('school')
  env.display.mockResolvedValueOnce({ customer: null })
  await contacts.show(ready)
  expect(contacts.error.value).toContain('配置格式错误')
  await contacts.show({ state: 'error', cause: new Error('准备失败') })
  expect(contacts.failureStage.value).toBe('prepare')
  await contacts.retry()
  expect(env.display).toHaveBeenCalledTimes(3)
})

it('downloads the server-selected path including revision and supports retry, preview and close', async () => {
  const { env, contacts } = setup()
  await contacts.show(ready)
  env.image.mockRejectedValueOnce(new Error('弱网'))
  await contacts.open('service')
  expect(contacts.imageError.value).toContain('弱网')
  expect(contacts.imageState.value).toBe('error')
  await contacts.retryImage()
  expect(env.image).toHaveBeenLastCalledWith('school', '/service?revision=1')
  expect(contacts.title.value).toBe('平台客服')
  expect(contacts.path.value).toBe('/local-image')
  await contacts.preview()
  expect(env.preview).toHaveBeenCalledWith('/local-image')
  env.preview.mockRejectedValueOnce(new Error('预览失败'))
  await contacts.preview()
  expect(env.notify).toHaveBeenCalledWith('图片预览失败，请重试')
  contacts.close()
  expect(contacts.selected.value).toBeUndefined()
  expect(contacts.path.value).toBe('')
})

it.each(['success', 'failure'])('keeps the last selection when the previous image returns %s', async outcome => {
  const { env, contacts } = setup()
  await contacts.show(ready)
  const first = deferred<string>()
  env.image.mockReturnValueOnce(first.promise)
  const pending = contacts.open('service')
  await contacts.open('service')
  expect(env.image).toHaveBeenCalledOnce()
  await contacts.open('join')
  if (outcome === 'success') first.resolve('/wrong')
  else first.reject(new Error('过期错误'))
  await pending
  expect(contacts.title.value).toBe('我要加盟')
  expect(contacts.path.value).toBe('/local-image')
  expect(contacts.imageState.value).toBe('ready')
  expect(env.failure).not.toHaveBeenCalled()
})

it.each(['close', 'missing', 'hide', 'dispose', 'scope', 'reload'])('invalidates late images after %s', async action => {
  const { env, contacts } = setup()
  await contacts.show(ready)
  const first = deferred<string>()
  env.image.mockReturnValueOnce(first.promise)
  const pending = contacts.open('service')
  if (action === 'close') contacts.close()
  else if (action === 'missing') await contacts.open('business')
  else if (action === 'hide') contacts.hide()
  else if (action === 'dispose') contacts.dispose()
  else if (action === 'reload') await contacts.show(ready)
  else { env.key.mockReturnValue('other'); contacts.reset() }
  first.resolve('/stale')
  await pending
  expect(contacts.path.value).toBe('')
  expect(contacts.selected.value).toBeUndefined()
  expect(env.failure).not.toHaveBeenCalled()
})

it.each(['hide', 'dispose', 'scope', 'newer'])('ignores late config success and failure after %s', async action => {
  for (const fails of [false, true]) {
    const { env, contacts } = setup()
    const first = deferred<Display>()
    env.display.mockReturnValueOnce(first.promise)
    const pending = contacts.show(ready)
    if (action === 'hide') contacts.hide()
    else if (action === 'dispose') contacts.dispose()
    else if (action === 'newer') await contacts.show(ready)
    else { env.key.mockReturnValue('other'); contacts.reset() }
    if (fails) first.reject(new Error('过期'))
    else first.resolve(display({ enabled: false }))
    await pending
    expect(contacts.state.value).toBe(action === 'newer' ? 'ready' : action === 'scope' ? 'error' : 'idle')
    expect(env.failure).not.toHaveBeenCalled()
  }
})

it('does not write error state back after error normalization clears the scope', async () => {
  const { env, contacts } = setup()
  env.failure.mockImplementation(() => { env.key.mockReturnValue('other'); contacts.reset(); return '拒绝访问' })
  env.display.mockRejectedValueOnce(new Error('403'))
  await contacts.show(ready)
  expect(contacts.failureStage.value).toBe('prepare')
  expect(contacts.error.value).toContain('状态已变化')
  contacts.dispose()
  await contacts.show(ready)
  expect(env.display).toHaveBeenCalledOnce()
})

it('uses the new revision after reentry and clears the old detail when configuration is disabled', async () => {
  const { env, contacts } = setup()
  await contacts.show(ready)
  await contacts.open('service')
  contacts.hide()
  env.display.mockResolvedValueOnce(display({ enabled: true, service: '/service?revision=2' }))
  await contacts.show(ready)
  expect(contacts.selected.value).toBeUndefined()
  await contacts.open('service')
  expect(env.image).toHaveBeenLastCalledWith('school', '/service?revision=2')
  env.display.mockResolvedValueOnce(display({ enabled: false }))
  await contacts.show(ready)
  expect(contacts.state.value).toBe('disabled')
  expect(contacts.path.value).toBe('')
  await contacts.open('service')
  expect(env.image).toHaveBeenCalledTimes(2)
})
