import { afterEach, expect, it, vi } from 'vitest'
const mocks = vi.hoisted(() => ({ scope: 'first', upload: vi.fn() }))
vi.mock('../src/features/miniapp-users/api', () => ({ miniappUserApi: { uploadAvatar: mocks.upload } }))
vi.mock('../src/features/school-context/context', () => ({ scopeKey: () => mocks.scope, onScopeChange: vi.fn() }))
import { uploadAvatar } from '../src/features/miniapp-users/avatar-files'
afterEach(() => { vi.unstubAllGlobals(); vi.clearAllMocks(); mocks.scope = 'first' })

it('does not upload a previous account photo after a scope change during file reading', async () => {
  let finish!: (value: { data: ArrayBuffer }) => void
  vi.stubGlobal('uni', { getFileSystemManager: () => ({ readFile: (options: { success: typeof finish }) => { finish = options.success } }) })
  const pending = uploadAvatar('/personal.png')
  mocks.scope = 'second'
  finish({ data: new ArrayBuffer(8) })
  await expect(pending).rejects.toThrow('登录状态已改变')
  expect(mocks.upload).not.toHaveBeenCalled()
})

it('rejects oversized files before starting an upload', async () => {
  vi.stubGlobal('uni', { getFileSystemManager: () => ({ readFile: (options: { success: (value: { data: ArrayBuffer }) => void }) => options.success({ data: new ArrayBuffer(5 * 1024 * 1024 + 1) }) }) })
  await expect(uploadAvatar('/large.png')).rejects.toThrow('5 MiB')
  expect(mocks.upload).not.toHaveBeenCalled()
})
