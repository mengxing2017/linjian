import { beforeEach, expect, it, vi } from 'vitest'
const state = vi.hoisted(() => ({ scope: 'account:1:school', changed: () => {} }))
vi.mock('../src/features/school-context/context', () => ({
  scopeKey: () => state.scope,
  onScopeChange: (fn: () => void) => {
    state.changed = fn
    return () => {}
  },
}))
vi.mock('../src/features/school-context/api', () => ({ schoolsApi: { branding: vi.fn() } }))
import { schoolsApi } from '../src/features/school-context/api'
import { brandingFile } from '../src/features/school-context/branding'
const unlink = vi.fn(),
  writeFile = vi.fn()
beforeEach(() => {
  state.changed()
  state.scope += ':next'
  vi.clearAllMocks()
  vi.stubGlobal('uni', {
    env: { USER_DATA_PATH: '/linjian-test' },
    getFileSystemManager: () => ({ unlink, writeFile }),
  })
  writeFile.mockImplementation(({ success }: { success: () => void }) => success())
  vi.mocked(schoolsApi.branding).mockResolvedValue(new ArrayBuffer(1))
})
it('shares a fetch in the same revision and removes cached files when the scope changes', async () => {
  const first = brandingFile('school', 'logo', 1)
  expect(brandingFile('school', 'logo', 1)).toBe(first)
  const path = await first
  expect(schoolsApi.branding).toHaveBeenCalledTimes(1)
  await brandingFile('school', 'logo', 2)
  expect(schoolsApi.branding).toHaveBeenCalledTimes(2)
  state.scope += ':changed'
  state.changed()
  expect(unlink).toHaveBeenCalledWith(expect.objectContaining({ filePath: path }))
  expect(unlink).toHaveBeenCalledTimes(2)
})
it('discards a response arriving after an account or school change', async () => {
  let complete!: (bytes: ArrayBuffer) => void
  vi.mocked(schoolsApi.branding).mockImplementation(
    () =>
      new Promise((resolve) => {
        complete = resolve
      }),
  )
  const pending = brandingFile('school', 'header', 1)
  state.scope += ':changed'
  state.changed()
  complete(new ArrayBuffer(1))
  await expect(pending).rejects.toThrow('学校或账号已变化')
  expect(writeFile).not.toHaveBeenCalled()
})
it('removes a file if its write completes after the scope changes', async () => {
  let complete!: () => void
  writeFile.mockImplementation(({ success }: { success: () => void }) => {
    complete = success
  })
  const pending = brandingFile('school', 'header', 1)
  await vi.waitFor(() => expect(writeFile).toHaveBeenCalledOnce())
  state.scope += ':changed'
  state.changed()
  complete()
  await expect(pending).rejects.toThrow('学校或账号已变化')
  expect(unlink).toHaveBeenCalledOnce()
})
