import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { permissionCatalog, rolePermissions } from '../../../test/permissions'
import { schoolDefaults } from '../../../test/school'
import { Members } from './members'

const api = vi.hoisted(() => ({ catalog: vi.fn(), mine: vi.fn(), members: vi.fn(), candidates: vi.fn(), detail: vi.fn(), save: vi.fn(), action: vi.fn(), recommendation: vi.fn() }))
vi.mock('./api', () => ({ browserClient: {}, createSchoolPermissionsClient: () => api }))
const member = { id:'operator', school_id:'school', username:'operator', display_name:'运营', active:true, role:'operator', mode:'school', revision:'1', catalog_version:'2026-11-20.2', overrides:[], effective_permissions:rolePermissions('operator') }
class ResizeObserver { observe() {} unobserve() {} disconnect() {} }
vi.stubGlobal('ResizeObserver', ResizeObserver)
Object.defineProperty(window,'matchMedia',{writable:true,value:()=>({matches:false,addListener:vi.fn(),removeListener:vi.fn(),addEventListener:vi.fn(),removeEventListener:vi.fn()})})
beforeEach(() => {
  vi.resetAllMocks()
  api.catalog.mockResolvedValue({...permissionCatalog,editable:true,uneditable_reason:''})
  api.mine.mockResolvedValue({effective_permissions:rolePermissions('admin')})
  api.members.mockResolvedValue({items:[{...member,platform_admin:false,override_count:0,updated_at:null}],page:{number:1,size:30,has_more:false}})
  api.detail.mockResolvedValue(member)
  api.candidates.mockResolvedValue({items:[]})
})
afterEach(cleanup)
async function open(platform = true) {
  render(<QueryClientProvider client={new QueryClient({defaultOptions:{queries:{retry:false}}})}><Members school={{...schoolDefaults,id:'school',name:'测试校',revision:1,config:{}}} platformAdmin={platform} currentUserId="root" refresh={async()=>{}} /></QueryClientProvider>)
  fireEvent.click(await screen.findByRole('button',{name:'查看 / 配置权限'}))
  return screen.findByRole('dialog')
}
it('renders all catalogue leaves, mutually exclusive overrides and dependency validation', async () => {
  await open()
  expect(screen.getAllByRole('checkbox').filter(input => input.getAttribute('aria-label')?.endsWith('单独允许'))).toHaveLength(45)
  const allow = screen.getByRole('checkbox',{name:'推荐/重推帖子单独允许'})
  const deny = screen.getByRole('checkbox',{name:'推荐/重推帖子单独禁止'})
  fireEvent.click(allow)
  expect(allow).toHaveProperty('checked',true)
  fireEvent.click(deny)
  expect(allow).toHaveProperty('checked',false)
  expect(deny).toHaveProperty('checked',true)
  fireEvent.click(screen.getByRole('checkbox',{name:'查看帖子单独禁止'}))
  expect(await screen.findByText('权限依赖未满足（不会自动补权）')).toBeTruthy()
  expect(api.save).not.toHaveBeenCalled()
})
it('keeps a conflict draft and never retries a versioned save automatically', async () => {
  api.save.mockRejectedValue(Object.assign(new Error('版本冲突'),{status:409}))
  await open()
  fireEvent.click(screen.getByRole('button',{name:'全部禁止'}))
  fireEvent.change(screen.getByRole('textbox',{name:'授权变更原因'}),{target:{value:'调整职责'}})
  fireEvent.click(screen.getByRole('button',{name:'预览差异并保存'}))
  fireEvent.click(await screen.findByRole('button',{name:'确认保存'}))
  await screen.findByText('配置已被其他人修改，草稿已保留，请重新加载比较后再提交。')
  expect(api.save).toHaveBeenCalledTimes(1)
  expect(api.save).toHaveBeenCalledWith('school','operator',expect.objectContaining({expected_revision:'1',reason:'调整职责',overrides:expect.any(Array)}))
  expect(screen.getByRole('textbox',{name:'授权变更原因'})).toHaveProperty('value','调整职责')
  expect(screen.getByRole('checkbox',{name:'查看帖子单独禁止'})).toHaveProperty('checked',true)
  api.detail.mockResolvedValue({...member,revision:'2'})
  fireEvent.click(screen.getByRole('button',{name:'读取最新配置'}))
  fireEvent.click(await screen.findByRole('button',{name:'以最新版本继续核对草稿'}))
  await waitFor(()=>expect(screen.getByText(/授权版本：2/)).toBeTruthy())
  expect(api.save).toHaveBeenCalledTimes(1)
})
it('limits school administrators to the recommendation override and never offers candidates', async () => {
  await open(false)
  expect(screen.queryByRole('button',{name:'添加后台成员'})).toBeNull()
  expect(screen.getByRole('checkbox',{name:'查看帖子单独允许'})).toHaveProperty('disabled',true)
  expect(screen.getByRole('checkbox',{name:'推荐/重推帖子单独允许'})).toHaveProperty('disabled',false)
  expect(screen.queryByRole('button',{name:'全部禁止'})).toBeNull()
  expect(api.candidates).not.toHaveBeenCalled()
})
it('explains platform identity instead of allowing ineffective school denials', async () => {
  api.detail.mockResolvedValue({...member,mode:'platform'})
  await open()
  expect(await screen.findByText('平台身份不受学校覆盖限制，不能配置学校 deny。')).toBeTruthy()
  expect(screen.getByRole('checkbox',{name:'查看帖子单独禁止'})).toHaveProperty('disabled',true)
  expect(screen.queryByRole('button',{name:'预览差异并保存'})).toBeNull()
})
