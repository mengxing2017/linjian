import { afterEach, expect, it, vi } from 'vitest'
import { nextTick } from 'vue'
const hooks = vi.hoisted(() => ({ onReady: vi.fn(), onShow: vi.fn(), onHide: vi.fn(), onPullDownRefresh: vi.fn(), onReachBottom: vi.fn(), onShareAppMessage: vi.fn(), onLoad: vi.fn() }))
const state = vi.hoisted(() => ({ user: null as null | { bound_school_id: string }, restore: vi.fn(), subscribe: vi.fn().mockReturnValue(() => {}) }))
vi.mock('@dcloudio/uni-app', () => hooks)
vi.mock('vue', async original => ({ ...await original<typeof import('vue')>(), onUnmounted: vi.fn() }))
vi.mock('../src/features/identity/session', () => ({ session: state, restoreSession: state.restore }))
vi.mock('../src/app/credentials', () => ({ credentials: { subscribe: state.subscribe } }))
import { useProtectedScreen } from '../src/app/page-lifecycle'
afterEach(() => { state.user = null; vi.clearAllMocks(); vi.unstubAllGlobals() })
it('does not mount or load protected content for a visitor and keeps public pages outside the guard', async () => {
  const launch = vi.fn().mockResolvedValue(undefined)
  vi.stubGlobal('uni', { reLaunch: launch })
  state.restore.mockResolvedValue(undefined)
  const { screen, allowed } = useProtectedScreen()
  const show = vi.fn()
  screen.value = { show }
  hooks.onShow.mock.calls[0][0]()
  await nextTick(); await nextTick()
  expect(allowed.value).toBe(false)
  expect(show).not.toHaveBeenCalled()
  expect(launch).toHaveBeenCalledWith({ url: '/pages/user/login' })
})
it('loads protected content only after the restored session has a bound school', async () => {
  vi.stubGlobal('uni', { reLaunch: vi.fn() })
  state.user = { bound_school_id: 'school' }
  state.restore.mockResolvedValue(undefined)
  const { screen, allowed } = useProtectedScreen()
  const show = vi.fn()
  screen.value = { show }
  hooks.onShow.mock.calls[0][0]()
  await nextTick(); await nextTick(); await nextTick()
  expect(allowed.value).toBe(true)
  expect(show).toHaveBeenCalledOnce()
})
