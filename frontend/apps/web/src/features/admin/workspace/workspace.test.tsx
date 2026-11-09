import {schoolDefaults} from '../../../test/school'
import { selectOption } from '../../../test/select-option'
import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, configure, fireEvent, render, screen, waitFor, within } from '@testing-library/react'
import { MemoryRouter, useLocation, useNavigate } from 'react-router-dom'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { Workspace } from './index'
import { identity, workspace, product } from './api'
vi.mock('../../school-context/api', async () => ({ schoolsApi: (await import('./api')).workspace }))
vi.mock('../../identity/api', async () => ({ identity: (await import('./api')).identity, subscribeUnauthorized: () => () => {} }))
vi.mock('./api', () => ({ product:{resources:{artwork:vi.fn().mockResolvedValue([])}}, identity: { session: vi.fn(), accounts: vi.fn(), create: vi.fn(), update: vi.fn(), reset: vi.fn(), logout: vi.fn() }, workspace: { schools: vi.fn(), cases: vi.fn(), resolveCase: vi.fn(), reviews: vi.fn(), review: vi.fn(), list: vi.fn(), schoolProfiles:{platform:vi.fn(),detail:vi.fn(),create:vi.fn(),update:vi.fn(),artwork:vi.fn(),branding:vi.fn()}, createSchool: vi.fn(), updateSchool: vi.fn(), mutate: vi.fn() } }))
class TestResizeObserver { observe() {} unobserve() {} disconnect() {} }
vi.stubGlobal('ResizeObserver', TestResizeObserver)
configure({ asyncUtilTimeout: 5000 })
afterEach(() => { cleanup(); vi.resetAllMocks() })
import {beforeEach} from 'vitest'
beforeEach(()=>{vi.mocked(workspace.cases).mockImplementation((s,q,signal)=>workspace.list(s,'cases',q,signal) as ReturnType<typeof workspace.cases>);vi.mocked(workspace.schoolProfiles.detail).mockImplementation(async id => (await workspace.schools()).find(s => s.id === id)!);vi.mocked(workspace.reviews).mockImplementation((s,q,signal)=>workspace.list(s,'posts',q,signal) as ReturnType<typeof workspace.reviews>);vi.mocked(product.resources.artwork).mockResolvedValue([]);vi.mocked(workspace.schoolProfiles.platform).mockImplementation(async(filters={})=>{const all=await workspace.schools();const filtered=all.filter(s=>!filters.search||s.name.includes(filters.search));const n=filters.page??1;return {items:filtered.slice((n-1)*10,n*10),page:{number:n,size:10,has_more:n*10<filtered.length}}})})
Object.defineProperty(window, 'matchMedia', { writable: true, value: (query: string) => ({ matches: false, media: query, onchange: null, addListener: vi.fn(), removeListener: vi.fn(), addEventListener: vi.fn(), removeEventListener: vi.fn(), dispatchEvent: vi.fn() }) })
it('restores the selected school and business page from its URL', async () => {
  vi.mocked(identity.session).mockResolvedValue({csrf:'csrf',user:{id:'u',username:'admin',display_name:'管理员',platform_admin:true,bound_school_id:null,grants:[],wecom_authenticated:false}})
  vi.mocked(workspace.schools).mockResolvedValue([{id:'school',name:'测试学校',...schoolDefaults,revision:1,config:{}}])
  vi.mocked(workspace.list).mockResolvedValue({items:[], page:{number:1,size:30,has_more:false}})
  vi.mocked(identity.accounts).mockResolvedValue([])
  const client = new QueryClient({defaultOptions:{queries:{retry:false}}})
  render(<QueryClientProvider client={client}><MemoryRouter initialEntries={['/admin?school=school&view=posts']}><Workspace/></MemoryRouter></QueryClientProvider>)
  const groupsTitle = await screen.findByText('内容审核', { selector: 'h1' })
  expect(within(groupsTitle.parentElement!).getByRole('heading', { name: '内容审核', level: 1 })).toBe(groupsTitle)
  await waitFor(() => expect(workspace.list).toHaveBeenCalledWith('school','posts',expect.any(String),expect.any(AbortSignal)))
  fireEvent.click(within(screen.getByRole('menu')).getByRole('menuitem', { name: '学校管理' }))
  const schoolsTitle = await screen.findByText('学校管理', { selector: 'h1' })
  expect(within(schoolsTitle.parentElement!).getByRole('heading', { name: '学校管理', level: 1 })).toBe(schoolsTitle)
  expect(screen.queryByLabelText('当前学校')).toBeNull()
  fireEvent.click(screen.getByRole('button', { name: '新建学校' }))
  expect(await screen.findByLabelText('学校名称')).toBeTruthy()
})

function LocationProbe() {
  const location = useLocation()
  const navigate = useNavigate()
  return <><output data-testid="location">{location.search}</output><button onClick={() => navigate(-1)}>返回上一页</button></>
}

function showManagement(view: string, platformAdmin = true, role: 'member' | 'operator' | 'admin' = 'admin') {
  vi.mocked(identity.session).mockResolvedValue({csrf:'csrf',user:{id:'u',username:'root',display_name:'Root',platform_admin:platformAdmin,bound_school_id:null,grants:[{school_id:'school',role}],wecom_authenticated:false}})
  vi.mocked(workspace.schools).mockResolvedValue([{id:'school',name:'甲学校',...schoolDefaults,revision:1,config:{}}, {id:'other',name:'乙学校',...schoolDefaults,revision:1,config:{}}])
  vi.mocked(workspace.list).mockResolvedValue({items:[]})
  vi.mocked(identity.accounts).mockResolvedValue([
    {id:'u',username:'root',display_name:'平台维护者',platform_admin:true,active:true},
    {id:'ordinary',username:'ordinary',display_name:'本地管理员',platform_admin:false,active:true,wecom_userid:'wx-user'},
  ])
  const client = new QueryClient({defaultOptions:{queries:{retry:false}}})
  render(<QueryClientProvider client={client}><MemoryRouter initialEntries={[`/admin?school=school&view=${view}`]}><Workspace/><LocationProbe/></MemoryRouter></QueryClientProvider>)
}

it.each(['comments', 'posts&subview=comments'])('opens comment reviews within content moderation from %s', async view => {
  showManagement(view, false, 'operator')
  vi.mocked(workspace.list).mockResolvedValue({items:[{id:'comment-1',post_title:'评论所属帖子',content:'评论正文',author_name:'评论用户昵称',audit_status:'pending',revision:1}]})
  expect(await screen.findByRole('tab', { name: '评论审核', selected: true })).toBeTruthy()
  expect(screen.getByRole('heading', { name: '内容审核', level: 1 })).toBeTruthy()
  expect(within(screen.getByRole('menu')).queryByRole('menuitem', { name: '评论审核' })).toBeNull()
  expect(screen.getByTestId('location').textContent).toBe('?school=school&view=posts&subview=comments')
  await waitFor(() => expect(workspace.list).toHaveBeenCalledWith('school', 'comments', expect.stringContaining('status=pending'), expect.any(AbortSignal)))
  const commentRow = within((await screen.findByText('评论正文')).closest('tr')!)
  expect(screen.getAllByRole('columnheader').map(header => header.textContent)).toEqual([
    '标题 / 名称', '审核状态', '内容', '用户昵称', '创建时间', '操作',
  ])
  expect(screen.getByRole('columnheader', { name: '用户昵称' })).toBeTruthy()
  expect(commentRow.getByText('评论用户昵称')).toBeTruthy()
  fireEvent.click(commentRow.getByRole('button', { name: /详\s*情/ }))
  const detail = await screen.findByRole('dialog')
  expect(within(detail).getByText('审核状态')).toBeTruthy()
  expect(within(detail).getByText('用户昵称')).toBeTruthy()
  expect(within(detail).getByText('评论用户昵称')).toBeTruthy()
  fireEvent.click(within(detail).getByRole('button', { name: 'Close' }))
  await waitFor(() => expect(detail.isConnected).toBe(false))
  fireEvent.click(screen.getByRole('tab', { name: '帖子审核' }))
  expect(screen.getByTestId('location').textContent).toBe('?school=school&view=posts&subview=posts')
  fireEvent.click(screen.getByRole('button', { name: '返回上一页' }))
  expect(await screen.findByRole('tab', { name: '评论审核', selected: true })).toBeTruthy()
})

it('keeps the legacy comment review URL subject to school management permission', async () => {
  showManagement('comments', false, 'member')
  expect(await screen.findByText('当前账号没有此功能的访问权限')).toBeTruthy()
  expect(screen.queryByRole('tab', { name: '评论审核' })).toBeNull()
  expect(workspace.list).not.toHaveBeenCalled()
})

it('separates ordinary accounts and administrators by privilege, not display name or current school', async () => {
  showManagement('accounts')
  expect(await screen.findByText('ordinary')).toBeTruthy()
  expect(screen.queryByLabelText('当前学校')).toBeNull()
  expect(screen.queryByText('平台维护者')).toBeNull()
  expect(screen.queryByText('新建学校', { exact: true })).toBeNull()
  fireEvent.click(within(screen.getByRole('menu')).getByText('平台管理员', { exact: true }))
  const adminRow = (await screen.findByText('平台维护者')).closest('tr')!
  expect(adminRow).toBeTruthy()
  expect(screen.queryByText('ordinary')).toBeNull()
  expect(screen.queryByText('新建账号', { exact: true })).toBeNull()
  expect(within(adminRow).queryByRole('button', {name:'编辑账号'})).toBeNull()
})

it('requires a new session after resetting the current platform administrator password', async () => {
  showManagement('administrators')
  const adminRow = (await screen.findByText('平台维护者')).closest('tr')!
  fireEvent.click(within(adminRow).getByRole('button', {name:'重置密码'}))
  expect(await screen.findByText(/这是当前登录账号/)).toBeTruthy()
  fireEvent.change(screen.getByLabelText('新密码（至少12个字符）'), {target:{value:'changed-password'}})
  vi.mocked(identity.reset).mockResolvedValue(undefined as never)
  vi.mocked(identity.session).mockRejectedValue(new Error('expired'))
  fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', {name:'确认重置密码'}))
  await waitFor(() => expect(identity.reset).toHaveBeenCalledWith('u','changed-password'))
  expect(await screen.findByText('登录工作空间', {selector:'h1'})).toBeTruthy()
})

it('searches and paginates schools, and opens settings for the selected row', async () => {
  showManagement('schools')
  vi.mocked(workspace.schools).mockResolvedValue(Array.from({length:12}, (_, i) => ({id:`s${i}`,name:`学校-${i}`,...schoolDefaults,revision:1,config:{}})))
  expect(await screen.findByText('学校-0')).toBeTruthy()
  expect(screen.queryByText('学校-11')).toBeNull()
  fireEvent.click(screen.getByRole('button',{name:'下一页'}))
  expect(await screen.findByText('学校-11')).toBeTruthy()
  fireEvent.change(screen.getByLabelText('搜索学校'), {target:{value:'学校-11'}})
  const row = (await screen.findByText('学校-11')).closest('tr')!
  fireEvent.click(within(row).getByRole('button', {name:'学校配置'}))
  expect(await screen.findByLabelText('学校名称')).toHaveProperty('value','学校-11')
  expect(screen.getByText('学校-11 · 学校设置')).toBeTruthy()
})

it('shows account grants across schools and saves global account state without a school id', async () => {
  showManagement('accounts')
  vi.mocked(workspace.list).mockImplementation(async school => ({items:[{id:'ordinary',role: school === 'school' ? 'admin' : 'operator'}]}))
  // Scope role queries to the account row/dialog: scanning the entire Ant Design
  // workspace repeatedly computes accessible names and styles for every button.
  const row = within((await screen.findByText('ordinary')).closest('tr')!)
  fireEvent.click(row.getByRole('button', {name:/详\s*情/}))
  expect(await screen.findByText('甲学校 · 学校管理员')).toBeTruthy()
  expect(await screen.findByText('乙学校 · 运营')).toBeTruthy()
  const details = screen.getByRole('dialog')
  fireEvent.click(within(details).getByRole('button', {name:'Close'}))
  await waitFor(() => expect(details.isConnected).toBe(false))
  fireEvent.click(row.getByRole('button', {name:'编辑账号'}))
  const editor = within(screen.getByRole('dialog'))
  await selectOption(editor.getByLabelText('账号状态（全平台）'), '停用')
  fireEvent.click(editor.getByRole('button', {name:/^保\s*存$/}))
  await waitFor(() => expect(identity.update).toHaveBeenCalledWith('ordinary', {display_name:'本地管理员',active:false}))
})

it.each(['member','operator'] as const)('blocks school settings and member administration for %s even through a direct URL', async role => {
  showManagement('settings',false,role)
  await screen.findByLabelText('当前学校')
  await waitFor(() => expect(workspace.schools).toHaveBeenCalled())
  expect(screen.queryByRole('menuitem', {name:'学校设置'})).toBeNull()
  expect(screen.queryByRole('menuitem', {name:'学校成员与权限'})).toBeNull()
  expect(screen.queryByLabelText('学校名称')).toBeNull()
  expect(screen.queryByRole('menuitem', {name:'登录账号管理'})).toBeNull()
})

it('allows a school administrator to read members without offering grant changes', async () => {
  showManagement('members',false)
  expect(await screen.findByText('学校授权由平台管理员维护。')).toBeTruthy()
  expect(screen.queryByRole('button',{name:'保存所选成员授权'})).toBeNull()
  expect(identity.accounts).not.toHaveBeenCalled()
  await waitFor(() => expect(workspace.list).toHaveBeenCalledWith('school','members',expect.any(String),expect.any(AbortSignal)))
})

it('shows newly created unassigned accounts directly in school members and grants the selected school role', async () => {
  showManagement('members')
  let granted = false
  vi.mocked(workspace.list).mockImplementation(async () => ({items:granted ? [{id:'ordinary',username:'ordinary',display_name:'本地管理员',role:'operator'}] : []}))
  vi.mocked(workspace.mutate).mockImplementation(async () => { granted = true; return {id:'ordinary'} })
  const row = (await screen.findByText('ordinary')).closest('tr')!
  expect(within(row).getByText('未授权')).toBeTruthy()
  expect(screen.getByText('平台维护者')).toBeTruthy()
  fireEvent.click(within(row).getByRole('checkbox'))
  fireEvent.click(screen.getByRole('button', {name:'保存所选成员授权'}))
  await waitFor(() => expect(workspace.mutate).toHaveBeenCalledWith('school','members/ordinary',{role:'operator'},'PUT'))
  await waitFor(() => expect(within(screen.getByText('ordinary').closest('tr')!).getByText('运营')).toBeTruthy())
})

it('allows explicit school administrator grants for platform accounts and refreshes session permissions', async () => {
  showManagement('members')
  vi.mocked(workspace.mutate).mockResolvedValue({id:'u'})
  const row=(await screen.findByText('平台维护者')).closest('tr')!
  fireEvent.click(within(row).getByRole('checkbox'))
  await selectOption(screen.getByRole('combobox',{name:'学校角色'}),'学校管理员')
  const calls=vi.mocked(identity.session).mock.calls.length
  fireEvent.click(screen.getByRole('button',{name:'保存所选成员授权'}))
  await waitFor(()=>expect(workspace.mutate).toHaveBeenCalledWith('school','members/u',{role:'admin'},'PUT'))
  await waitFor(()=>expect(vi.mocked(identity.session).mock.calls.length).toBeGreaterThan(calls))
})




it('keeps school settings scoped after moving them into platform functions', async () => {
  showManagement('settings', false, 'admin')
  expect(await screen.findByLabelText('学校名称')).toBeTruthy()
  expect(screen.getByLabelText('当前学校')).toBeTruthy()
  expect(screen.queryByText('范围：全平台')).toBeNull()
})

it('rejects direct global identity URLs without fetching global accounts', async () => {
  showManagement('accounts', false, 'operator')
  expect(await screen.findByText('当前账号没有此功能的访问权限')).toBeTruthy()
  expect(identity.accounts).not.toHaveBeenCalled()
})

it('shows a clear error for unknown navigation instead of an unrelated page', async () => {
  showManagement('does-not-exist')
  expect(await screen.findByText('页面不存在')).toBeTruthy()
  expect(workspace.list).not.toHaveBeenCalled()
})

it('explains missing school management permission for an ordinary member at the default entry', async () => {
  showManagement('', false, 'member')
  expect(await screen.findByText('当前账号没有该学校的后台管理权限，请联系平台管理员。')).toBeTruthy()
  expect(screen.queryByText('页面不存在')).toBeNull()
  expect(workspace.list).not.toHaveBeenCalled()
})

it('resolves a school report with its current revision', async () => {
  showManagement('cases', false, 'operator')
  vi.mocked(workspace.list).mockResolvedValue({ items: [{ id: 'case-1', title: '帖子标题', reporter_name: '举报者', school_name: '甲学校', kind: 'report', content: '待处理举报', images: [], state: 'open', revision: 4, handled_at: null }] })
  const row = within((await screen.findByText('待处理举报')).closest('tr')!)
  fireEvent.click(row.getByRole('button', { name: /详\s*情/ }))
  const detail = within(await screen.findByRole('dialog'))
  fireEvent.change(detail.getByLabelText('处理说明'), { target: { value: '已核查处理' } })
  await selectOption(detail.getByLabelText('处理结果'), '已处理')
  fireEvent.click(detail.getByRole('button', { name: /^保\s*存$/ }))
  await waitFor(() => expect(workspace.resolveCase).toHaveBeenCalledWith('school', 'case-1', {
    resolution: '已核查处理', state: 'resolved', revision: 4,
  }))
})

it('keeps platform default resources read-only while allowing edits to school-owned resources', async () => {
  showManagement('resources', false, 'admin')
  vi.mocked(workspace.list).mockResolvedValue({ items: [
    { id: 'global-resource', title: '平台默认资源', active: true, school_id: null },
    { id: 'school-resource', title: '本校资源', active: true, school_id: 'school' },
  ] })
  const globalRow = within((await screen.findByText('平台默认资源', { exact: true })).closest('tr')!)
  const schoolRow = within(screen.getByText('本校资源', { exact: true }).closest('tr')!)
  expect(globalRow.queryByRole('button', { name: /^编\s*辑$/ })).toBeNull()
  expect(schoolRow.getByRole('button', { name: /^编\s*辑$/ })).toBeTruthy()
  fireEvent.click(globalRow.getByRole('button', { name: /详\s*情/ }))
  expect(within(await screen.findByRole('dialog')).queryByRole('button', { name: /^编\s*辑$/ })).toBeNull()
  expect(workspace.mutate).not.toHaveBeenCalled()
})

it('shows audit details without mutation actions', async () => {
  showManagement('audit', false, 'operator')
  vi.mocked(workspace.list).mockResolvedValue({ items: [{ id: 'audit-1', action: 'school.updated', detail: { revision: 3 } }] })
  const row = within((await screen.findByText('school.updated')).closest('tr')!)
  fireEvent.click(row.getByRole('button', { name: /详\s*情/ }))
  const detail = within(await screen.findByRole('dialog'))
  expect(detail.getByText(/"revision":3/)).toBeTruthy()
  expect(detail.queryByRole('button', { name: /^保\s*存$/ })).toBeNull()
  expect(workspace.mutate).not.toHaveBeenCalled()
})
