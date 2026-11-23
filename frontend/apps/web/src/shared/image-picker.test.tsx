import { afterEach, expect, it, vi } from 'vitest'
import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { ImagePicker } from './image-picker'

class TestResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}
vi.stubGlobal('ResizeObserver', TestResizeObserver)
afterEach(() => {
  cleanup()
  vi.resetAllMocks()
})

function mount(load: (page: number) => Promise<{ items: { id: string }[]; page?: { has_more: boolean } }>) {
  const onChange = vi.fn()
  render(
    <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>
      <ImagePicker
        scope="school"
        label="学校图标"
        value={null}
        onChange={onChange}
        load={load}
        upload={vi.fn()}
        source={(id) => `/assets/${id}`}
      />
    </QueryClientProvider>,
  )
  return onChange
}

it('hides page controls when the image library has no more pages', async () => {
  const load = vi.fn().mockResolvedValue({ items: [{ id: 'asset-1' }], page: { has_more: false } })
  mount(load)
  await waitFor(() => expect(load).toHaveBeenCalledWith(1))
  expect(screen.queryByRole('button', { name: /上一页/ })).toBeNull()
  expect(screen.queryByRole('button', { name: /下一页/ })).toBeNull()
  expect(screen.queryByText('第 1 页')).toBeNull()
})

it('hides page controls while the first page is loading', () => {
  mount(() => new Promise(() => {}))
  expect(screen.queryByText(/第 1 页/)).toBeNull()
  expect(screen.queryByRole('button', { name: /上一页|下一页/ })).toBeNull()
})

it('shows only the applicable controls when more pages exist', async () => {
  const load = vi
    .fn()
    .mockResolvedValueOnce({ items: [{ id: 'asset-1' }], page: { has_more: true } })
    .mockResolvedValueOnce({ items: [{ id: 'asset-2' }], page: { has_more: false } })
  mount(load)

  const next = await screen.findByRole('button', { name: '下一页 学校图标' })
  expect(screen.queryByRole('button', { name: /上一页/ })).toBeNull()
  expect(screen.getByText('第 1 页')).toBeTruthy()

  fireEvent.click(next)
  await waitFor(() => expect(load).toHaveBeenLastCalledWith(2))
  expect(await screen.findByRole('button', { name: '上一页 学校图标' })).toBeTruthy()
  expect(screen.getByText('第 2 页')).toBeTruthy()
  expect(screen.queryByRole('button', { name: /下一页/ })).toBeNull()
})

function uploadPicker(upload = vi.fn().mockResolvedValue({ id: 'uploaded' }), load = vi.fn().mockResolvedValue({ items: [] })) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const onChange = vi.fn(), onBusyChange = vi.fn()
  const view = (scope: string) => <QueryClientProvider client={client}>
    <ImagePicker scope={scope} label="二维码" value={null} onChange={onChange} onBusyChange={onBusyChange} load={load} upload={upload} source={id => `/assets/${id}`} />
  </QueryClientProvider>
  const result = render(view('school'))
  return { ...result, view, upload, onChange, onBusyChange }
}
function choosePicture() {
  fireEvent.change(document.querySelector('input[type=file]')!, { target: { files: [new File(['image'], 'qr.png', { type: 'image/png' })] } })
}

it('confirms accepted uploads through the real notification bridge without saving the form', async () => {
  const { upload, onChange } = uploadPicker()
  choosePicture()
  await screen.findByText('二维码已上传，保存表单后生效')
  expect(upload).toHaveBeenCalledTimes(1)
  expect(onChange).toHaveBeenCalledWith('uploaded')
  expect(screen.getByRole('button', { name: '上传二维码' })).toBeTruthy()
})

it('retains failed uploads for retry and never reports failure as success', async () => {
  const upload = vi.fn().mockRejectedValueOnce(new Error('上传被拒绝')).mockResolvedValue({ id: 'retried' })
  const { onChange } = uploadPicker(upload)
  choosePicture()
  await screen.findByText('上传被拒绝')
  expect(onChange).not.toHaveBeenCalled()
  expect(screen.queryByText('二维码已上传，保存表单后生效')).toBeNull()
  fireEvent.click(screen.getByRole('button', { name: '重试上传' }))
  await screen.findByText('二维码已上传，保存表单后生效')
  expect(onChange).toHaveBeenCalledWith('retried')
})

it('keeps an accepted upload when the library refresh fails and retries only reads', async () => {
  const load = vi.fn().mockResolvedValueOnce({ items: [] }).mockRejectedValueOnce(new Error('离线')).mockResolvedValue({ items: [] })
  const { upload, onChange } = uploadPicker(undefined, load)
  await waitFor(() => expect(load).toHaveBeenCalledTimes(1))
  choosePicture()
  await screen.findByText('图片已上传，素材列表刷新失败，请重新加载素材列表')
  expect(onChange).toHaveBeenCalledWith('uploaded')
  expect(screen.queryByRole('button', { name: '重试上传' })).toBeNull()
  fireEvent.click(screen.getByRole('button', { name: '重新加载素材列表' }))
  await waitFor(() => expect(load).toHaveBeenCalledTimes(3))
  expect(upload).toHaveBeenCalledTimes(1)
})

it.each(['scope', 'roundtrip', 'unmount'])('ignores upload completion after %s', async change => {
  let resolve!: (picture: { id: string }) => void
  const upload = vi.fn(() => new Promise<{ id: string }>(finish => { resolve = finish }))
  const { onChange, rerender, view, unmount, onBusyChange } = uploadPicker(upload)
  choosePicture()
  await waitFor(() => expect(upload).toHaveBeenCalledTimes(1))
  if (change !== 'unmount') {
    rerender(view('other-school'))
    if (change === 'roundtrip') rerender(view('school'))
  }
  else unmount()
  await act(async () => { resolve({ id: 'late' }) })
  expect(onChange).not.toHaveBeenCalled()
  expect(screen.queryByText('二维码已上传，保存表单后生效')).toBeNull()
  expect(onBusyChange).toHaveBeenLastCalledWith(false)
})

it('does not start duplicate uploads while a request is pending', async () => {
  let resolve!: (picture: { id: string }) => void
  const upload = vi.fn(() => new Promise<{ id: string }>(finish => { resolve = finish }))
  uploadPicker(upload)
  choosePicture()
  await waitFor(() => expect(upload).toHaveBeenCalledTimes(1))
  choosePicture()
  await act(async () => { resolve({ id: 'uploaded' }) })
  await screen.findByText('二维码已上传，保存表单后生效')
  expect(upload).toHaveBeenCalledTimes(1)
})

it('balances busy callbacks without decrementing again when an idle picker unmounts', async () => {
  const { unmount, onBusyChange } = uploadPicker()
  choosePicture()
  await waitFor(() => expect(onBusyChange).toHaveBeenLastCalledWith(false))
  expect(onBusyChange.mock.calls.map(([busy]) => busy)).toEqual([true, false])
  unmount()
  expect(onBusyChange.mock.calls.map(([busy]) => busy)).toEqual([true, false])
})
