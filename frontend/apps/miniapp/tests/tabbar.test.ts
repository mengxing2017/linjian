import { afterEach, expect, it, vi } from 'vitest'
import { effectScope, nextTick } from 'vue'
vi.mock('../src/features/identity/session', async () => {
  const { reactive } = await import('vue')
  return { session: reactive({ user: { id: 'member', bound_school_id: 'school' } as null | { id: string; bound_school_id: string } }) }
})
vi.mock('../src/features/school-context/context', async () => {
  const { reactive } = await import('vue')
  const schoolContext = reactive({ id: 'school', generation: 1 })
  return { schoolContext, scopeKey: () => `${schoolContext.id}:${schoolContext.generation}` }
})
vi.mock('../src/features/school-context/theme', async () => {
  const { ref } = await import('vue')
  return { schoolTheme: ref('--primary:#5b8ff9') }
})
vi.mock('../src/features/community/notification-state', async () => {
  const { reactive } = await import('vue')
  return { notificationSummary: { state: reactive({ count: 100 }), refresh: vi.fn().mockResolvedValue(undefined), sync: vi.fn() } }
})
vi.mock('../src/app/navigation', () => ({ openPublish: vi.fn().mockResolvedValue(undefined), switchRoot: vi.fn().mockResolvedValue(undefined) }))
import { installTabbar } from '../src/app/tabbar'
import { notificationSummary } from '../src/features/community/notification-state'
import { session } from '../src/features/identity/session'
import { schoolContext } from '../src/features/school-context/context'
import { schoolTheme } from '../src/features/school-context/theme'
import { openPublish, switchRoot } from '../src/app/navigation'

afterEach(() => vi.unstubAllGlobals())
it('bridges native instances to Vue state without duplicating business state or requests', async () => {
  let route = 'pages/index/index'
  vi.stubGlobal('getCurrentPages', () => [{ route }])
  vi.stubGlobal('uni', { showToast: vi.fn() })
  const scope = effectScope()
  const app = {} as Parameters<typeof installTabbar>[0]
  scope.run(() => installTabbar(app))
  const first = { setData: vi.fn() }, second = { setData: vi.fn() }
  app.linjianTabbar!.attach(first)
  expect(first.setData).toHaveBeenLastCalledWith(expect.objectContaining({ selected: 'home', ready: true, badge: '99+', theme: '--primary:#5b8ff9' }))
  expect(first.setData.mock.lastCall![0].items.map((item: { id: string }) => item.id)).toEqual(['home', 'following', 'publish', 'messages', 'profile'])
  route = 'pages/user/message'
  app.linjianTabbar!.attach(second)
  expect(second.setData).toHaveBeenLastCalledWith(expect.objectContaining({ selected: 'messages' }))
  notificationSummary.state.count = 1
  expect(first.setData).toHaveBeenLastCalledWith(expect.objectContaining({ badge: '1' }))
  app.linjianTabbar!.activate('publish')
  app.linjianTabbar!.activate('profile')
  app.linjianTabbar!.activate('https://evil.example')
  await nextTick()
  expect(openPublish).toHaveBeenCalledOnce()
  expect(switchRoot).toHaveBeenCalledExactlyOnceWith('profile')
  vi.mocked(notificationSummary.refresh).mockClear()
  schoolContext.generation++
  await nextTick()
  expect(notificationSummary.sync).toHaveBeenCalled()
  expect(notificationSummary.refresh).not.toHaveBeenCalled()
  session.user = null
  expect(first.setData).toHaveBeenLastCalledWith(expect.objectContaining({ ready: false }))
  app.linjianTabbar!.detach(first)
  app.linjianTabbar!.detach(second)
  scope.stop()
  expect(schoolTheme.value).toContain('--primary')
})
