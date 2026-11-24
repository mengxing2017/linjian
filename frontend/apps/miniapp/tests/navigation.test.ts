import { afterEach, beforeEach, expect, it, vi } from 'vitest'
const mocks = vi.hoisted(() => ({ session: { user: { id: 'account' } }, scope: 'scope-1', save: vi.fn(), clear: vi.fn() }))
vi.mock('../src/features/identity/session', () => ({ session: mocks.session }))
vi.mock('../src/features/school-context/context', () => ({ scopeKey: () => mocks.scope }))
vi.mock('../src/features/identity/return-target', () => ({ savePublishTarget: mocks.save, clearReturnTarget: mocks.clear }))
import { finishPublish, leavePublish, openPage, openPublish, publishOrigin, switchRoot } from '../src/app/navigation'
import { rootRoutes } from '../src/app/root-routes'

let pages: { route: string }[]
beforeEach(() => {
  mocks.scope = 'scope-1'
  pages = [{ route: 'pages/index/index' }]
  vi.stubGlobal('getCurrentPages', () => pages)
  vi.stubGlobal('uni', {
    switchTab: vi.fn().mockImplementation(async ({ url }) => { pages = [{ route: url.slice(1) }] }),
    navigateTo: vi.fn().mockImplementation(async ({ url }) => { pages.push({ route: url.slice(1) }) }),
    navigateBack: vi.fn().mockImplementation(async () => { pages.pop() }),
    redirectTo: vi.fn().mockResolvedValue(undefined),
  })
})
afterEach(() => vi.unstubAllGlobals())
it('registers four destinations and routes existing links through switchTab', async () => {
  expect(rootRoutes.map(root => root.id)).toEqual(['home', 'following', 'messages', 'profile'])
  await switchRoot('home')
  expect(uni.switchTab).not.toHaveBeenCalled()
  await openPage('/pages/user/message')
  expect(uni.switchTab).toHaveBeenCalledWith({ url: '/pages/user/message' })
  expect(uni.navigateTo).not.toHaveBeenCalled()
  await openPage('/pages/index/detail?school=s&id=p')
  expect(uni.navigateTo).toHaveBeenCalledWith({ url: '/pages/index/detail?school=s&id=p' })
})
it.each(['home', 'following', 'messages', 'profile'] as const)('returns a cancelled editor to its %s source', async root => {
  await switchRoot(root)
  await openPublish()
  expect(publishOrigin()).toBe(root)
  expect(mocks.save).toHaveBeenLastCalledWith(root, 'account')
  await leavePublish()
  expect(uni.navigateBack).toHaveBeenCalledWith({ delta: 1 })
  expect(pages[0].route).toBe(rootRoutes.find(item => item.id === root)!.path.slice(1))
})
it('falls back to home after scope loss and never trusts an arbitrary destination', async () => {
  await switchRoot('profile'); await openPublish()
  mocks.scope = 'new-scope'
  await leavePublish()
  expect(uni.switchTab).toHaveBeenLastCalledWith({ url: '/pages/index/index' })
  await expect(switchRoot('https://evil.example' as 'home')).rejects.toThrow()
})
it('coalesces rapid taps and releases the navigation lock after a failure', async () => {
  vi.mocked(uni.navigateTo).mockRejectedValueOnce(new Error('failed'))
  const first = openPublish()
  expect(openPublish()).toBe(first)
  await expect(first).rejects.toThrow('failed')
  await switchRoot('messages')
  expect(uni.switchTab).toHaveBeenCalledOnce()
})
it('replaces the editor with a detail page while retaining the source root', async () => {
  await switchRoot('messages'); await openPublish()
  await finishPublish('school', 'post')
  expect(uni.redirectTo).toHaveBeenCalledWith({ url: '/pages/index/detail?school=school&id=post' })
})
it('creates a home root before opening a directly entered successful post', async () => {
  mocks.scope = 'direct-entry'
  pages = [{ route: 'pages/index/publish' }]
  await finishPublish('school', 'post')
  expect(uni.switchTab).toHaveBeenCalledWith({ url: '/pages/index/index' })
  expect(uni.navigateTo).toHaveBeenCalledWith({ url: '/pages/index/detail?school=school&id=post' })
})
