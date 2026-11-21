import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { PermissionFixture } from '../../../test/permissions'
import { OperationServices } from './services'
import { OperationReports } from './reports'
import { SchoolMarketing } from './marketing'
import { serviceApi } from './api'
vi.mock('./api',()=>({serviceApi:{list:vi.fn(),options:vi.fn(),preview:vi.fn(),save:vi.fn(),marketingSettings:vi.fn(),marketingSetting:vi.fn()}}))
vi.mock('./media-picker',()=>({MediaPicker:()=> <div>本校服务素材</div>}))
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
it('does not expose official ad unit editing to school managers',async()=>{
  show(<SchoolMarketing school="school"/>,'operator')
  await screen.findByText('学校官方单元')
  expect(screen.getByRole('switch')).toHaveProperty('disabled',true)
  expect(screen.queryByLabelText('官方广告单元 ID')).toBeNull()
})
