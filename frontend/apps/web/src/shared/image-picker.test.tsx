import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
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
