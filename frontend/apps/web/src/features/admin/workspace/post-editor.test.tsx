import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { PostEditor } from './post-editor'
import { workspace, type PostManagementView } from './api'

vi.mock('./api', async original => ({
  ...await original<typeof import('./api')>(),
  workspace: { upload: vi.fn(), postManagement: { create: vi.fn(), update: vi.fn(), detail: vi.fn() } },
}))
afterEach(() => { cleanup(); vi.resetAllMocks() })

function show(id?: string) {
  const saved = vi.fn().mockResolvedValue(undefined)
  const close = vi.fn()
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
    <PostEditor school="school" id={id} saved={saved} close={close} />
  </QueryClientProvider>)
  return { saved, close }
}

it('creates a body-only post without expanding the optional title', async () => {
  show()
  expect(screen.queryByRole('textbox', { name: '标题（选填）' })).toBeNull()
  fireEvent.change(screen.getByLabelText('正文'), { target: { value: ' 后台直接表达 ' } })
  fireEvent.click(screen.getByRole('button', { name: '保存帖子' }))
  await waitFor(() => expect(workspace.postManagement.create).toHaveBeenCalledWith('school', {
    title: '', content: '后台直接表达', images: [], tags: [],
  }))
})

it('clears an existing title explicitly while retaining the current revision and body', async () => {
  vi.mocked(workspace.postManagement.detail).mockResolvedValue({
    id: 'post', title: '原标题', display_label: '原标题', content: '原正文', images: [], tags: [], revision: 7,
  } as unknown as PostManagementView)
  show('post')
  expect(await screen.findByRole('textbox', { name: '标题（选填）' })).toHaveProperty('value', '原标题')
  fireEvent.click(screen.getByRole('button', { name: '移除标题' }))
  fireEvent.click(screen.getByRole('button', { name: '保存帖子' }))
  await waitFor(() => expect(workspace.postManagement.update).toHaveBeenCalledWith('school', 'post', {
    title: '', content: '原正文', images: [], tags: [], revision: 7,
  }))
})

it('confirms image upload separately from saving the post', async () => {
  vi.mocked(workspace.upload).mockResolvedValue({ url: '/image.png' })
  show()
  fireEvent.change(document.querySelector('input[type=file]')!, { target: { files: [new File(['image'], 'photo.png', { type: 'image/png' })] } })
  await screen.findByText('图片已上传，提交帖子后生效')
  expect(workspace.upload).toHaveBeenCalledTimes(1)
  expect(workspace.postManagement.create).not.toHaveBeenCalled()
  expect(screen.getByRole('button', { name: '保存帖子' })).toBeTruthy()
})

it('retains the post editor without reporting success on failed upload', async () => {
  vi.mocked(workspace.upload).mockRejectedValueOnce(new Error('图片上传失败'))
  show()
  fireEvent.change(screen.getByLabelText('正文'), { target: { value: '草稿内容' } })
  fireEvent.change(document.querySelector('input[type=file]')!, { target: { files: [new File(['image'], 'photo.png', { type: 'image/png' })] } })
  await screen.findByText('图片上传失败')
  expect(screen.queryByText('图片已上传，提交帖子后生效')).toBeNull()
  expect(screen.getByLabelText('正文')).toHaveProperty('value', '草稿内容')
})
