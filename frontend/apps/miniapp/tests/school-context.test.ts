import { beforeEach, expect, it, vi } from 'vitest'
import type { School } from '../src/features/school-context/api'

const mocks = vi.hoisted(() => ({
  revision: 0,
  changed: () => {},
  restore: vi.fn(),
  detail: vi.fn(),
  list: vi.fn(),
  session: { user: { bound_school_id: 'school' } as { bound_school_id: string | null } | null },
}))
vi.mock('../src/app/credentials', () => ({ credentials: {
  get revision() { return mocks.revision },
  subscribe: (listener: () => void) => { mocks.changed = listener },
} }))
vi.mock('../src/app/transport', () => ({ cancelRequests: vi.fn() }))
vi.mock('../src/features/identity/session', () => ({ session: mocks.session, restoreSession: mocks.restore }))
vi.mock('../src/features/school-context/api', () => ({ schoolsApi: { detail: mocks.detail, list: mocks.list } }))

import { invalidateSchool, loadSchool, requireSchool, schoolContext } from '../src/features/school-context/context'

const school: School = {
  id: 'school', name: '学校', short_name: '', province: '', city: '', area: '',
  latitude: 31, longitude: 121, status: 'normal', revision: 1,
  logo_url: null, header_bg_image_url: null, theme_primary: '', theme_secondary: '', theme_text_color: '',
}

beforeEach(() => {
  vi.resetAllMocks()
  mocks.session.user = { bound_school_id: 'school' }
  mocks.restore.mockResolvedValue(undefined)
  mocks.detail.mockResolvedValue(school)
  mocks.changed()
})

it('loads only the bound school after session restoration without fetching any school list', async () => {
  let restore!: () => void
  let finish!: (school: School) => void
  mocks.restore.mockReturnValueOnce(new Promise<void>(resolve => { restore = resolve }))
  mocks.detail.mockReturnValueOnce(new Promise<School>(resolve => { finish = resolve }))
  const pending = loadSchool()
  expect(schoolContext.loaded).toBe(false)
  expect(mocks.detail).not.toHaveBeenCalled()
  restore()
  await vi.waitFor(() => expect(mocks.detail).toHaveBeenCalledWith('school'))
  expect(schoolContext.loaded).toBe(false)
  finish(school)
  expect(await pending).toBe(true)
  expect(schoolContext.loaded).toBe(true)
  expect(schoolContext.school).toEqual(school)
  expect(schoolContext.id).toBe('school')
  expect(mocks.list).not.toHaveBeenCalled()
})

it('shares concurrent requests for the same bound school', async () => {
  let finish!: (school: School) => void
  mocks.detail.mockReturnValueOnce(new Promise<School>(resolve => { finish = resolve }))
  const first = loadSchool()
  const second = loadSchool()
  await vi.waitFor(() => expect(mocks.detail).toHaveBeenCalledOnce())
  finish(school)
  expect(await Promise.all([first, second])).toEqual([true, true])
})

it.each([null, { bound_school_id: null }])('does not choose a fallback school for an unbound user: %s', async user => {
  mocks.session.user = user
  expect(await loadSchool()).toBe(false)
  expect(mocks.detail).not.toHaveBeenCalled()
  expect(mocks.list).not.toHaveBeenCalled()
  expect(schoolContext.school).toBeUndefined()
})

it('rejects a cross-school link without changing the bound school or fetching another school', async () => {
  await loadSchool()
  mocks.detail.mockClear()
  await expect(requireSchool('other-school')).rejects.toThrow('只能访问当前绑定学校')
  expect(mocks.detail).not.toHaveBeenCalled()
  expect(schoolContext.id).toBe('school')
  expect(schoolContext.school).toEqual(school)
})

it('rejects a detail response that does not match the session binding', async () => {
  mocks.detail.mockResolvedValue({ ...school, id: 'other-school' })
  await expect(loadSchool()).rejects.toThrow('学校资料与绑定学校不一致')
  expect(schoolContext.loaded).toBe(false)
  expect(schoolContext.id).toBe('')
})

it('preserves the unavailable-school warning without selecting a different school', async () => {
  mocks.detail.mockResolvedValue({ ...school, status: 'hidden' })
  await expect(requireSchool()).rejects.toThrow('学校暂不可用，请联系管理员')
  expect(schoolContext.loaded).toBe(true)
  expect(schoolContext.school).toBeUndefined()
  expect(schoolContext.id).toBe('')
  expect(mocks.list).not.toHaveBeenCalled()
})

it('keeps network errors distinct from confirmed unavailability and allows retry', async () => {
  mocks.detail.mockRejectedValueOnce(new Error('网络错误'))
  await expect(loadSchool()).rejects.toThrow('网络错误')
  expect(schoolContext.loaded).toBe(false)
  expect(await loadSchool()).toBe(true)
  expect(schoolContext.loaded).toBe(true)
})

it('clears the previous unavailable result while retrying', async () => {
  mocks.detail.mockResolvedValueOnce({ ...school, status: 'hidden' })
  await loadSchool()
  expect(schoolContext.loaded).toBe(true)
  let finish!: (school: School) => void
  mocks.detail.mockReturnValueOnce(new Promise<School>(resolve => { finish = resolve }))
  const pending = loadSchool()
  await vi.waitFor(() => expect(schoolContext.loaded).toBe(false))
  finish(school)
  await pending
  expect(schoolContext.id).toBe('school')
})

it.each(['account', 'binding', 'invalidation'])('ignores school details arriving after %s changes', async change => {
  let finish!: (school: School) => void
  mocks.detail.mockReturnValueOnce(new Promise<School>(resolve => { finish = resolve }))
  const pending = loadSchool()
  await vi.waitFor(() => expect(mocks.detail).toHaveBeenCalledOnce())
  if (change === 'account') { mocks.revision++; mocks.changed() }
  else if (change === 'binding') mocks.session.user = { bound_school_id: 'new-school' }
  else invalidateSchool()
  finish(school)
  expect(await pending).toBe(false)
  expect(schoolContext.loaded).toBe(false)
  expect(schoolContext.school).toBeUndefined()
})

it('refreshes school branding and invalidates scoped data when the school revision changes', async () => {
  await loadSchool()
  const previous = schoolContext.generation
  mocks.detail.mockResolvedValueOnce({ ...school, revision: 2, theme_primary: '#123456' })
  await loadSchool()
  expect(schoolContext.generation).toBe(previous + 1)
  expect(schoolContext.school?.theme_primary).toBe('#123456')
})
