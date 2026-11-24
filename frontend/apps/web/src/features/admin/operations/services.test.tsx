import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { act, cleanup, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { PermissionFixture } from '../../../test/permissions'
import { OperationServices } from './services'
import { OperationReports } from './reports'
import { SchoolMarketing } from './marketing'
import { serviceApi } from './api'
vi.mock('./api',()=>({serviceApi:{list:vi.fn(),options:vi.fn(),preview:vi.fn(),save:vi.fn(),override:vi.fn(),marketingSettings:vi.fn(),marketingSetting:vi.fn()}}))
vi.mock('./media-picker',()=>({MediaPicker:({onBusyChange}:{onBusyChange?:(busy:boolean)=>void})=> <div>本校服务素材<button onClick={()=>onBusyChange?.(true)}>模拟图片上传</button></div>}))
vi.mock('../../school-context',()=>({useSchools:()=>({data:[{id:'school',name:'学校'}]})}))
class Resize { observe(){} unobserve(){} disconnect(){} }
vi.stubGlobal('ResizeObserver',Resize)
Object.defineProperty(window,'matchMedia',{writable:true,value:()=>({matches:false,addListener:vi.fn(),removeListener:vi.fn(),addEventListener:vi.fn(),removeEventListener:vi.fn()})})
beforeEach(()=>{
  vi.clearAllMocks()
  vi.mocked(serviceApi.list).mockResolvedValue({id:'current',token:'1:0',items:[{id:'entry',revision:1,title:'校园图书服务',subtitle:'查询图书',badge:'',image_ref:{kind:'asset',id:'image'},category_id:null,cooperation_id:null,action:{kind:'none'},active:true,featured:true,sort_order:0,starts_at:null,ends_at:null}]})
  vi.mocked(serviceApi.options).mockResolvedValue({id:'current',targets:[],categories:[],cooperations:[]})
  vi.mocked(serviceApi.preview).mockResolvedValue({id:'current',token:'1:0',server_time:'2026-11-21T00:00:00Z',items:[],featured:[],categories:[],has_more:false,preview:true})
  vi.mocked(serviceApi.marketingSettings).mockResolvedValue({id:'current',items:[{id:'slot',name:'学校官方单元',format:'custom',available:true,enabled:false,revision:0}]})
})
afterEach(cleanup)
function show(node:React.ReactNode,role='operator'){render(<PermissionFixture role={role}><QueryClientProvider client={new QueryClient({defaultOptions:{queries:{retry:false}}})}>{node}</QueryClientProvider></PermissionFixture>)}
it('keeps school operators read-only without hiding the new service capability',async()=>{
  show(<OperationServices school="school"/>)
  await screen.findByText('校园图书服务')
  expect(screen.queryByRole('button',{name:'新增服务入口'})).toBeNull()
  fireEvent.click(screen.getByRole('button',{name:/查\s*看/}))
  const drawer=within(await screen.findByRole('dialog'))
  expect(drawer.queryByRole('button',{name:/保\s*存/})).toBeNull()
  expect(drawer.getByLabelText('入口名称').closest('fieldset')).toHaveProperty('disabled',true)
  expect(serviceApi.save).not.toHaveBeenCalled()
})
it('does not implicitly grant report export to ordinary school administration',()=>{
  show(<OperationReports school="school"/>,'admin')
  expect(screen.getByRole('button',{name:'生成报告'})).toBeTruthy()
  expect(screen.queryByRole('button',{name:'导出 CSV'})).toBeNull()
})
it.each([true, false])('explains report dates and queryable history when partial=%s', async partial => {
  const result: OperationReport = {
    id: 'report',
    report: {
      pagination: { page: 1, size: 30, total: 0 },
      coverage: { available_from: '2026-10-31T16:00:00Z', requested_from: '2026-10-31T16:00:00Z', partial, notice: '没有记录不代表没有访问' },
      starts_at: '2026-10-31T16:00:00Z', ends_at: '2026-11-01T16:00:00Z', timezone: 'Asia/Shanghai',
      notice: '按北京时间每天汇总。接口调用成功，不代表已成交或服务已完成。',
      rows: [], summary: { views: 0, clicks: 0, ctr: null }, trends: [],
    },
  }
  vi.mocked(serviceApi.report).mockResolvedValueOnce(result)
  show(<OperationReports />)
  expect(screen.getByText('按北京时间每天统计。点击或操作成功，不代表已成交或服务已完成。')).toBeTruthy()
  expect(screen.queryByText(/本次报告生成后 10 分钟/)).toBeNull()
  expect(screen.getByText(/结束日当天不计入统计/)).toBeTruthy()
  fireEvent.change(screen.getByLabelText('开始日'), { target: { value: '2026-11-01' } })
  fireEvent.change(screen.getByLabelText('结束日（不包含）'), { target: { value: '2026-11-02' } })
  fireEvent.mouseDown(screen.getByRole('combobox'))
  fireEvent.click(screen.getByText('首页横幅'))
  fireEvent.click(screen.getByRole('button', { name: '生成报告' }))
  await screen.findByText(result.report.notice)
  expect(serviceApi.report).toHaveBeenCalledWith(undefined, '2026-11-01T00:00:00+08:00', '2026-11-02T00:00:00+08:00', expect.objectContaining({ placement: 'home_banner' }))
  expect(screen.getByText(/可查询的数据从 2026-11-01 00:00（北京时间）开始/)).toHaveProperty('textContent', expect.stringContaining(partial ? '所选日期中有部分时间没有历史数据。' : '所选日期均在可查询的时间范围内。'))
  expect(screen.getByText('本次报告生成后 10 分钟内可查看和导出，过期后请重新生成。')).toBeTruthy()
  expect(screen.getAllByRole('columnheader', { name: '接口调用成功' }).length).toBeGreaterThan(0)
  expect(screen.queryByText(/上海自然日|请求覆盖|报告快照/)).toBeNull()
})
it('does not expose official ad unit editing to school managers',async()=>{
  show(<SchoolMarketing school="school"/>,'operator')
  await screen.findByText('学校官方单元')
  const control = screen.getByRole('switch', { name: '启用营销位：学校官方单元' })
  expect(control).toHaveProperty('disabled',true)
  fireEvent.click(control)
  expect(serviceApi.marketingSetting).not.toHaveBeenCalled()
  expect(screen.queryByLabelText('官方广告单元 ID')).toBeNull()
})

it('names each marketing switch and preserves immediate scoped updates and unavailable states', async () => {
  const items = [
    { id: 'first', name: '校园服务', format: 'custom', available: true, enabled: false, revision: 7 },
    { id: 'second', name: '平台停用单元', format: 'custom', available: false, enabled: false, revision: 3 },
  ]
  vi.mocked(serviceApi.marketingSettings).mockResolvedValueOnce({ id: 'current', items })
    .mockResolvedValue({ id: 'current', items: [{ ...items[0], enabled: true }, items[1]] })
  vi.mocked(serviceApi.marketingSetting).mockResolvedValue({ id: 'first', revision: 8, enabled: true })
  show(<SchoolMarketing school="school" />, 'admin')
  const editable = await screen.findByRole('switch', { name: '启用营销位：校园服务' })
  const unavailable = screen.getByRole('switch', { name: '启用营销位：平台停用单元' })
  expect(editable.getAttribute('aria-checked')).toBe('false')
  expect(unavailable).toHaveProperty('disabled', true)
  fireEvent.click(unavailable)
  expect(serviceApi.marketingSetting).not.toHaveBeenCalled()
  fireEvent.click(editable)
  await waitFor(() => expect(serviceApi.marketingSetting).toHaveBeenCalledExactlyOnceWith('school', 'first', 7, true))
  await waitFor(() => expect(editable.getAttribute('aria-checked')).toBe('true'))
})

it('keeps a named marketing switch unchanged when an immediate update fails', async () => {
  vi.mocked(serviceApi.marketingSetting).mockRejectedValueOnce(new Error('保存冲突'))
  show(<SchoolMarketing school="school" />, 'admin')
  const control = await screen.findByRole('switch', { name: '启用营销位：学校官方单元' })
  fireEvent.click(control)
  await screen.findByText('保存冲突')
  expect(serviceApi.marketingSetting).toHaveBeenCalledExactlyOnceWith('school', 'slot', 0, true)
  expect(control.getAttribute('aria-checked')).toBe('false')
  expect(serviceApi.marketingSettings).toHaveBeenCalledOnce()
})

const entities = [
  ['shortcuts', 'home-shortcuts', '服务入口'],
  ['shortcuts', 'operation-service-categories', '服务分类'],
  ['targets', 'operation-service-targets', '服务目标'],
  ['targets', 'operation-partners', '合作方'],
  ['targets', 'operation-cooperations', '合作项目'],
  ['marketing', 'marketing-slots', '官方营销位'],
] as const

it.each(entities)('confirms newly created %s / %s and closes its editor', async (mode, entity, name) => {
  vi.mocked(serviceApi.list).mockResolvedValue({ id: 'current', token: '1:0', items: [] })
  show(<OperationServices mode={mode} />, 'admin')
  if (name !== entities.find(row => row[0] === mode)?.[2]) fireEvent.click(screen.getByRole('tab', { name }))
  fireEvent.click(screen.getByRole('button', { name: `新增${name}` }))
  fireEvent.click(within(await screen.findByRole('dialog')).getByRole('button', { name: /^保\s*存$/ }))
  await screen.findByText(`${name}已创建`)
  expect(serviceApi.save).toHaveBeenCalledWith(undefined, entity, expect.any(Object), undefined)
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
})

it.each(entities)('confirms edits to %s / %s without claiming approval', async (mode, entity, name) => {
  const entry = { id: 'entry', revision: 1, name: '配置', title: '配置', active: true, sort_order: 0, image_ref: { kind: 'asset', id: 'image' }, action: { kind: 'none' }, channels: [], versions: [{ id: 'version', revision: 1, state: 'draft', config: { name: '配置', channels: [] } }] }
  vi.mocked(serviceApi.list).mockResolvedValue({ id: 'current', token: '1:0', items: [entry] } as Awaited<ReturnType<typeof serviceApi.list>>)
  show(<OperationServices mode={mode} />, 'admin')
  if (name !== entities.find(row => row[0] === mode)?.[2]) fireEvent.click(screen.getByRole('tab', { name }))
  fireEvent.click(await screen.findByRole('button', { name: entity === 'operation-service-targets' ? '修改并生成新版本' : /^编\s*辑$/ }))
  fireEvent.click(within(await screen.findByRole('dialog')).getByRole('button', { name: /^保\s*存$/ }))
  await screen.findByText(entity === 'operation-service-targets' ? '服务目标新版本已保存' : `${name}已保存`)
  expect(serviceApi.save).toHaveBeenCalledWith(undefined, entity, expect.any(Object), 'entry')
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
})

it('keeps failed service saves editable without reporting success', async () => {
  vi.mocked(serviceApi.save).mockRejectedValueOnce(new Error('保存冲突'))
  show(<OperationServices />, 'admin')
  fireEvent.click(screen.getByRole('button', { name: '新增服务入口' }))
  fireEvent.change(screen.getByLabelText('入口名称'), { target: { value: '保留草稿' } })
  fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: /^保\s*存$/ }))
  await screen.findByText('保存冲突')
  expect(screen.getByLabelText('入口名称')).toHaveProperty('value', '保留草稿')
  expect(screen.queryByText('服务入口已创建')).toBeNull()
})

it('reports a completed service write separately from a failed list refresh', async () => {
  show(<OperationServices />, 'admin')
  await screen.findByText('校园图书服务')
  fireEvent.click(screen.getByRole('button', { name: '新增服务入口' }))
  vi.mocked(serviceApi.list).mockRejectedValue(new Error('列表不可用'))
  fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: /^保\s*存$/ }))
  await screen.findByText('服务入口已创建')
  await screen.findByText('操作已完成，列表刷新失败，请刷新查看')
  expect(screen.queryByRole('dialog')).toBeNull()
  vi.mocked(serviceApi.list).mockResolvedValue({ id: 'current', token: '1:0', items: [] })
  fireEvent.click(screen.getByRole('button', { name: /^刷\s*新$/ }))
  expect(serviceApi.save).toHaveBeenCalledTimes(1)
})

it('confirms a school override and closes only after the write succeeds', async () => {
  vi.mocked(serviceApi.preview).mockResolvedValue({ id: 'current', token: '1:0', server_time: '2026-11-21T00:00:00Z', items: [{ id: 'default', title: '平台入口', school_id: null, override_revision: 0, revision: 1, subtitle: '', badge: '', image_url: '/icon', category_id: null, category_name: null, featured: true, sort_order: 0, cooperation_id: null, starts_at: null, ends_at: null, action: { kind: 'none' }, channels: [], disclosure: null, cooperation: null }], featured: [], categories: [], has_more: false, preview: true })
  show(<OperationServices school="school" />, 'admin')
  fireEvent.click(await screen.findByRole('button', { name: '展示覆盖' }))
  fireEvent.click(within(await screen.findByRole('dialog')).getByRole('button', { name: '保存覆盖' }))
  await screen.findByText('本校覆盖已保存')
  expect(serviceApi.override).toHaveBeenCalledWith('school', 'default', 0, '1:0', {})
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull())
})

it('does not submit service writes twice and ignores completion after closing the editor', async () => {
  let resolve!: (value: Awaited<ReturnType<typeof serviceApi.save>>) => void
  vi.mocked(serviceApi.save).mockReturnValueOnce(new Promise(finish => { resolve = finish }))
  show(<OperationServices />, 'admin')
  fireEvent.click(screen.getByRole('button', { name: '新增服务入口' }))
  const drawer = within(screen.getByRole('dialog'))
  const save = drawer.getByRole('button', { name: /^保\s*存$/ })
  fireEvent.click(save)
  fireEvent.click(save)
  expect(serviceApi.save).toHaveBeenCalledTimes(1)
  fireEvent.click(drawer.getByRole('button', { name: 'Close' }))
  await act(async () => { resolve({ id: 'late', revision: 1, name: '配置', active: true, sort_order: 0 }) })
  expect(screen.queryByText('服务入口已创建')).toBeNull()
})

it('retains a rejected override without a success message', async () => {
  vi.mocked(serviceApi.preview).mockResolvedValue({ id: 'current', token: '1:0', server_time: '2026-11-21T00:00:00Z', items: [{ id: 'default', title: '平台入口', school_id: null, override_revision: 0, revision: 1, subtitle: '', badge: '', image_url: '/icon', category_id: null, category_name: null, featured: true, sort_order: 0, cooperation_id: null, starts_at: null, ends_at: null, action: { kind: 'none' }, channels: [], disclosure: null, cooperation: null }], featured: [], categories: [], has_more: false, preview: true })
  vi.mocked(serviceApi.override).mockRejectedValueOnce(new Error('覆盖冲突'))
  show(<OperationServices school="school" />, 'admin')
  fireEvent.click(await screen.findByRole('button', { name: '展示覆盖' }))
  fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: '保存覆盖' }))
  await screen.findByText('覆盖冲突')
  expect(screen.getByRole('dialog')).toBeTruthy()
  expect(screen.queryByText('本校覆盖已保存')).toBeNull()
})

it('blocks final service saves while images are uploading', async () => {
  show(<OperationServices />, 'admin')
  fireEvent.click(screen.getByRole('button', { name: '新增服务入口' }))
  const drawer = within(screen.getByRole('dialog'))
  fireEvent.click(drawer.getByRole('button', { name: '模拟图片上传' }))
  const save = drawer.getByRole('button', { name: /^保\s*存$/ })
  expect(save).toHaveProperty('disabled', true)
  fireEvent.click(save)
  expect(serviceApi.save).not.toHaveBeenCalled()
})
