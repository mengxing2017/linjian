import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { CustomerConfiguration } from './customer'
import { operationsApi } from './api'
import type { CustomerConfig } from './api'

vi.mock('./api', () => ({ operationsApi: { customer: vi.fn(), saveCustomer: vi.fn() } }))
vi.mock('./media-picker', () => ({ MediaPicker: ({ label, onChange, onBusyChange }: { label: string; onChange: (id: string | null) => void; onBusyChange?: (busy: boolean) => void }) => <div><button onClick={() => onChange('image')}>{label}</button><button onClick={() => onBusyChange?.(true)}>上传{label}</button></div> }))

const initial: CustomerConfig = { id: 'current', revision: 1, enabled: true, service_asset_id: null, join_asset_id: null, business_asset_id: null, fallback_asset_id: null, note: '原始提示' }
beforeEach(() => {
  vi.mocked(operationsApi.customer).mockReset().mockResolvedValue(initial)
  vi.mocked(operationsApi.saveCustomer).mockReset().mockResolvedValue({ ...initial, revision: 2 })
})
afterEach(cleanup)
function show() {
  render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><CustomerConfiguration /></QueryClientProvider>)
}

it('explains both pages, allows empty images and refreshes server-normalized values after save', async () => {
  vi.mocked(operationsApi.customer).mockResolvedValueOnce(initial).mockResolvedValue({ ...initial, revision: 2, note: '规范化提示' })
  show()
  await screen.findByText(/平台客服、我要加盟、商务合作未配置联系方式/)
  expect(screen.getByText(/“我的”和“个人信息”/)).toBeTruthy()
  fireEvent.click(screen.getByRole('button', { name: '保存配置' }))
  await screen.findByDisplayValue('规范化提示')
  expect(operationsApi.saveCustomer).toHaveBeenCalledWith(expect.objectContaining({ enabled: true, revision: 1, fallback_asset_id: null }))
  expect(screen.queryByText('申请加盟二维码')).toBeNull()
})

it('computes dedicated/fallback sources and shows only missing entries without blocking save', async () => {
  vi.mocked(operationsApi.customer).mockResolvedValue({ ...initial, service_asset_id: 'dedicated' })
  show()
  await screen.findByText('平台客服图片来源：专属')
  expect(screen.getByText(/我要加盟、商务合作未配置联系方式/)).toBeTruthy()
  fireEvent.click(screen.getByRole('button', { name: '通用兜底二维码' }))
  expect(screen.getByText('我要加盟图片来源：通用')).toBeTruthy()
  expect(screen.queryByText(/未配置联系方式；/)).toBeNull()
  fireEvent.click(screen.getByRole('switch'))
  expect(screen.getByText(/三个联系入口将隐藏/)).toBeTruthy()
})

it('blocks writes during image upload', async () => {
  show()
  fireEvent.click(await screen.findByRole('button', { name: '上传平台客服二维码' }))
  expect(screen.getByRole('button', { name: '保存配置' })).toHaveProperty('disabled', true)
  fireEvent.click(screen.getByRole('button', { name: '保存配置' }))
  expect(operationsApi.saveCustomer).not.toHaveBeenCalled()
})

it('requires explicit confirmed reload on revision conflict and never repeats the stale write', async () => {
  vi.mocked(operationsApi.saveCustomer).mockRejectedValueOnce(Object.assign(new Error('冲突'), { status: 409 }))
  show()
  const input = await screen.findByDisplayValue('原始提示')
  fireEvent.change(input, { target: { value: '本地草稿' } })
  fireEvent.click(screen.getByRole('button', { name: '保存配置' }))
  await screen.findByText(/配置已被其他管理员更新/)
  expect(screen.getByDisplayValue('本地草稿')).toBeTruthy()
  expect(screen.getByRole('button', { name: '保存配置' })).toHaveProperty('disabled', true)
  fireEvent.click(screen.getByRole('button', { name: '重新加载配置' }))
  fireEvent.click(await screen.findByRole('button', { name: /取\s*消/ }))
  expect(operationsApi.customer).toHaveBeenCalledOnce()
  vi.mocked(operationsApi.customer).mockResolvedValue({ ...initial, revision: 3, note: '他人更新' })
  fireEvent.click(screen.getByRole('button', { name: '重新加载配置' }))
  fireEvent.click(await screen.findByRole('button', { name: /确\s*认/ }))
  await screen.findByDisplayValue('他人更新')
  expect(operationsApi.saveCustomer).toHaveBeenCalledOnce()
})

it('retries only reads when saving succeeded but the subsequent refresh failed', async () => {
  vi.mocked(operationsApi.customer).mockResolvedValueOnce(initial).mockRejectedValueOnce(new Error('读取失败')).mockResolvedValue({ ...initial, revision: 2 })
  show()
  fireEvent.click(await screen.findByRole('button', { name: '保存配置' }))
  await screen.findByText('配置已保存，读取最新配置失败，请重试读取')
  expect(screen.getByRole('button', { name: '保存配置' })).toHaveProperty('disabled', true)
  fireEvent.click(screen.getByRole('button', { name: '重试读取最新配置' }))
  await waitFor(() => expect(screen.getByRole('button', { name: '保存配置' })).toHaveProperty('disabled', false))
  expect(operationsApi.saveCustomer).toHaveBeenCalledOnce()
  expect(operationsApi.customer).toHaveBeenCalledTimes(3)
})

it('provides a retry when the initial configuration read fails', async () => {
  vi.mocked(operationsApi.customer).mockRejectedValueOnce(new Error('初始读取失败'))
  show()
  fireEvent.click(await screen.findByRole('button', { name: '重试读取配置' }))
  await screen.findByDisplayValue('原始提示')
  expect(operationsApi.saveCustomer).not.toHaveBeenCalled()
})
