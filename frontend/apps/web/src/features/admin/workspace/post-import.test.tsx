import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { PostImport } from './post-import'
import { workspace, type PostImportPreview } from './api'
vi.mock('./api', () => ({
  workspace: { postManagement: { preview: vi.fn(), importDetail: vi.fn(), commit: vi.fn() } },
}))
class ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}
vi.stubGlobal('ResizeObserver', ResizeObserver)
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: () => ({
    matches: false,
    addListener() {},
    removeListener() {},
    addEventListener() {},
    removeEventListener() {},
  }),
})
const preview: PostImportPreview = {
  id: 'job',
  actor_id: 'root',
  school_id: 'school',
  revision: 1,
  digest: 'digest',
  rows: [{ row: 2, post: { title: '导入帖子', content: '正文', images: [], tags: [] } }],
  errors: [],
  state: 'preview',
  result: [],
  expires_at: '2026-11-10T03:00:00Z',
}
afterEach(() => {
  cleanup()
  vi.resetAllMocks()
  sessionStorage.clear()
})
it('shows row errors and prevents a partial import', async () => {
  vi.mocked(workspace.postManagement.preview).mockResolvedValue({
    ...preview,
    errors: [{ row: 3, message: '标题不能为空' }],
  })
  render(<PostImport school="school" close={() => {}} saved={async () => {}} />)
  const input = document.querySelector('input[type=file]')!
  fireEvent.change(input, {
    target: { files: [new File(['标题,正文\na,b'], 'posts.csv', { type: 'text/csv' })] },
  })
  await screen.findByText('标题不能为空')
  expect(screen.getByRole('button', { name: '确认全部导入' }).hasAttribute('disabled')).toBe(true)
  expect(workspace.postManagement.commit).not.toHaveBeenCalled()
})
it('recovers an uncertain job and retries with the original idempotency key', async () => {
  sessionStorage.setItem(
    'linjian:post-import:root:school',
    JSON.stringify({ id: 'job', key: 'original-key', committing: true }),
  )
  vi.mocked(workspace.postManagement.importDetail).mockResolvedValue(preview)
  vi.mocked(workspace.postManagement.commit)
    .mockRejectedValueOnce(Object.assign(new Error('网络中断'), { status: 0 }))
    .mockResolvedValueOnce({
      ...preview,
      revision: 2,
      rows: [],
      state: 'committed',
      result: ['post-id'],
    })
  const saved = vi.fn().mockResolvedValue(undefined)
  render(<PostImport school="school" owner="root" close={() => {}} saved={saved} />)
  const retry = await screen.findByRole('button', { name: '重试同一次导入' })
  fireEvent.click(retry)
  await screen.findByText('网络中断')
  fireEvent.click(screen.getByRole('button', { name: '重试同一次导入' }))
  await screen.findByText('导入完成，共 1 条帖子')
  await waitFor(() => expect(saved).toHaveBeenCalledTimes(1))
  expect(workspace.postManagement.commit).toHaveBeenNthCalledWith(1, 'school', 'job', {
    revision: 1,
    digest: 'digest',
    idempotency_key: 'original-key',
  })
  expect(workspace.postManagement.commit).toHaveBeenNthCalledWith(2, 'school', 'job', {
    revision: 1,
    digest: 'digest',
    idempotency_key: 'original-key',
  })
})

it('allows a new file after an uncertain recovered job has expired', async () => {
  sessionStorage.setItem(
    'linjian:post-import:root:school',
    JSON.stringify({ id: 'old-job', key: 'old-key', committing: true }),
  )
  vi.mocked(workspace.postManagement.importDetail).mockResolvedValue({
    ...preview,
    id: 'old-job',
    state: 'expired',
    rows: [],
  })
  vi.mocked(workspace.postManagement.preview).mockResolvedValue(preview)
  render(<PostImport school="school" owner="root" close={() => {}} saved={async () => {}} />)
  await screen.findByText('预览已过期，请重新选择文件')
  const input = document.querySelector<HTMLInputElement>('input[type=file]')!
  await waitFor(() => expect(input.disabled).toBe(false))
  fireEvent.change(input, {
    target: { files: [new File(['标题,正文\na,b'], 'posts.csv', { type: 'text/csv' })] },
  })
  await waitFor(() => expect(workspace.postManagement.preview).toHaveBeenCalledTimes(1))
  expect(workspace.postManagement.commit).not.toHaveBeenCalled()
})
