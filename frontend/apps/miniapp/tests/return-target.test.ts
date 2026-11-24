import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { clearReturnTarget, readReturnTarget, saveReturnTarget, savePublishTarget, targetPath } from '../src/features/identity/return-target'
const mocks = vi.hoisted(() => ({ load: vi.fn(), restore: vi.fn(), session: { user: null as unknown } }))
vi.mock('../src/features/school-context/context', () => ({ loadSchool: mocks.load, scopeKey: () => 'scope' }))
vi.mock('../src/features/identity/session', () => ({ restoreSession: mocks.restore, session: mocks.session }))
import { resumeReturnTarget } from '../src/features/identity/resume-target'
const school = '40000000-0000-0000-0000-000000000001'
const id = '40000000-0000-0000-0000-000000000002'
let saved: unknown
beforeEach(() => {
  saved = undefined
  vi.stubGlobal('uni', {
    getStorageSync: vi.fn(() => saved),
    setStorageSync: vi.fn((_key, value) => { saved = value }),
    removeStorageSync: vi.fn(() => { saved = undefined }),
    reLaunch: vi.fn().mockResolvedValue(undefined),
    switchTab: vi.fn().mockResolvedValue(undefined),
    navigateTo: vi.fn().mockResolvedValue(undefined),
  })
  vi.stubGlobal('getCurrentPages', () => [{ route: 'pages/user/login' }])
  mocks.session.user = null
  mocks.load.mockReset()
  mocks.restore.mockReset()
})
afterEach(() => { vi.unstubAllGlobals(); vi.useRealTimers() })
it('only reconstructs a fixed school-scoped detail path and expires stale targets', () => {
  expect(() => saveReturnTarget('https://evil.example', id)).toThrow()
  expect(() => saveReturnTarget(school, '../other')).toThrow()
  saveReturnTarget(school, id)
  expect(targetPath(readReturnTarget()!)).toBe(`/pages/index/detail?school=${school}&id=${id}`)
  vi.useFakeTimers()
  vi.setSystemTime(Date.now() + 31 * 60 * 1000)
  expect(readReturnTarget()).toBeUndefined()
  expect(saved).toBeUndefined()
})
it('waits for login and school authorization and coalesces simultaneous resumption', async () => {
  saveReturnTarget(school, id)
  expect(await resumeReturnTarget()).toBe(false)
  expect(uni.reLaunch).not.toHaveBeenCalled()
  mocks.session.user = { id: 'member' }
  mocks.load.mockResolvedValue(false)
  expect(await resumeReturnTarget()).toBe(false)
  expect(readReturnTarget()).toBeDefined()
  mocks.load.mockResolvedValue(true)
  expect(await Promise.all([resumeReturnTarget(), resumeReturnTarget()])).toEqual([true, true])
  expect(mocks.load).toHaveBeenLastCalledWith(school)
  expect(uni.reLaunch).toHaveBeenCalledOnce()
  expect(saved).toBeUndefined()
})
it('clears a denied target and supports explicit cancellation', async () => {
  saveReturnTarget(school, id)
  mocks.session.user = { id: 'member' }
  mocks.load.mockRejectedValue(new Error('school denied'))
  await expect(resumeReturnTarget()).rejects.toThrow('school denied')
  expect(saved).toBeUndefined()
  expect(uni.reLaunch).not.toHaveBeenCalled()
  saveReturnTarget(school, id)
  clearReturnTarget()
  expect(readReturnTarget()).toBeUndefined()
})
it('resumes a whitelisted publish intent through its root after school authorization', async () => {
  savePublishTarget('messages')
  expect(targetPath(readReturnTarget()!)).toBe('/pages/index/publish')
  mocks.session.user = { id: 'member' }
  mocks.load.mockResolvedValue(true)
  expect(await resumeReturnTarget()).toBe(true)
  expect(uni.switchTab).toHaveBeenCalledWith({ url: '/pages/user/message' })
  expect(uni.navigateTo).toHaveBeenCalledWith({ url: '/pages/index/publish' })
  expect(mocks.load).toHaveBeenCalledWith(undefined)
  expect(saved).toBeUndefined()
})
it('discards another account publish intent and rejects external destinations', async () => {
  savePublishTarget('profile', 'someone-else')
  mocks.session.user = { id: 'member' }
  expect(await resumeReturnTarget()).toBe(false)
  expect(saved).toBeUndefined()
  expect(uni.navigateTo).not.toHaveBeenCalled()
  saved = { kind: 'publish', origin: 'https://evil.example', createdAt: Date.now() }
  expect(readReturnTarget()).toBeUndefined()
  expect(() => savePublishTarget('https://evil.example' as 'home')).toThrow()
})
