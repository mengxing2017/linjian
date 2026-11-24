import { PermissionFixture } from '../../../test/permissions'
import {afterEach,expect,it,vi} from 'vitest'
import {cleanup,fireEvent,render as renderView,screen,waitFor} from '@testing-library/react'
import {QueryClient,QueryClientProvider} from '@tanstack/react-query'
import {SchoolForm,Schools} from './school-profile'
import {workspace,type School} from './api'
import {schoolDefaults} from '../../../test/school'
vi.mock('./api',()=>({workspace:{schoolProfiles:{create:vi.fn(),update:vi.fn(),detail:vi.fn(),platform:vi.fn(),branding:vi.fn(),artwork:vi.fn()}},product:{resources:{artwork:vi.fn().mockResolvedValue({items:[]}),upload:vi.fn()}}}))
afterEach(()=>{cleanup();vi.clearAllMocks()})
const school:School={...schoolDefaults,id:'school',name:'示例学校',revision:3,config:{post_audit_mode:'manual',comment_audit_mode:'manual'}}
function mount(platform:boolean,existing?:School){render(<QueryClientProvider client={new QueryClient({defaultOptions:{queries:{retry:false}}})}><SchoolForm school={existing} platform={platform} onSaved={async()=>{}}/></QueryClientProvider>)}
it('creates with basics only and keeps theme defaults in the payload',async()=>{
 mount(true)
 for(const label of ['名称','简称','省份 / 城市 / 区域','纬度','经度','状态'])expect(screen.getByLabelText(label)).toBeTruthy()
 expect(screen.queryByLabelText('主题主色')).toBeNull()
 expect(screen.queryByLabelText('图标')).toBeNull()
 expect(screen.getByText(/图标、背景图与主题色可在创建后完善/)).toBeTruthy()
 fireEvent.change(screen.getByLabelText('名称'),{target:{value:'新学校'}})
 fireEvent.change(screen.getByLabelText('纬度'),{target:{value:'0'}})
 fireEvent.change(screen.getByLabelText('经度'),{target:{value:'0'}})
 fireEvent.click(screen.getByRole('button',{name:'创建学校'}))
 await waitFor(()=>expect(workspace.schoolProfiles.create).toHaveBeenCalledWith(expect.objectContaining({name:'新学校',status:'normal',revision:0,theme_primary:'#6A5AF9'})),{timeout:5000})
})
it('requires both coordinates and does not permit clearing them',async()=>{
 mount(true)
 fireEvent.change(screen.getByLabelText('名称'),{target:{value:'新学校'}})
 fireEvent.change(screen.getByLabelText('纬度'),{target:{value:'23.1'}})
 fireEvent.click(screen.getByRole('button',{name:'创建学校'}))
 expect(await screen.findByText('请填写经度',{}, {timeout:5000})).toBeTruthy()
 expect(workspace.schoolProfiles.create).not.toHaveBeenCalled()
})
it('submits the selected province, city and area names',async()=>{
 mount(true)
 fireEvent.change(screen.getByLabelText('名称'),{target:{value:'新学校'}})
 fireEvent.change(screen.getByLabelText('纬度'),{target:{value:'23.1'}})
 fireEvent.change(screen.getByLabelText('经度'),{target:{value:'113.3'}})
 const region=screen.getByLabelText('省份 / 城市 / 区域')
 fireEvent.mouseDown(region)
 fireEvent.click(region)
 fireEvent.click(await screen.findByText('广东省'))
 fireEvent.click(await screen.findByText('广州市'))
 fireEvent.click(await screen.findByText('天河区'))
 fireEvent.click(screen.getByRole('button',{name:'创建学校'}))
 await waitFor(()=>expect(workspace.schoolProfiles.create).toHaveBeenCalledWith(expect.objectContaining({name:'新学校',province:'广东省',city:'广州市',area:'天河区'})),{timeout:5000})
})
it('requires missing historical coordinates before saving an edit',async()=>{
 mount(true,{...school,latitude:null,longitude:null})
 fireEvent.click(screen.getByRole('button',{name:'保存资料'}))
 expect(await screen.findByText('请填写纬度',{}, {timeout:5000})).toBeTruthy()
 expect(await screen.findByText('请填写经度',{}, {timeout:5000})).toBeTruthy()
 expect(workspace.schoolProfiles.update).not.toHaveBeenCalled()
})
it('keeps the existing standard region when editing',async()=>{
 mount(true,{...school,province:'广东省',city:'广州市',area:'天河区'})
 fireEvent.click(screen.getByRole('button',{name:'保存资料'}))
 await waitFor(()=>expect(workspace.schoolProfiles.update).toHaveBeenCalledWith('school',expect.objectContaining({province:'广东省',city:'广州市',area:'天河区',revision:3})),{timeout:5000})
})
it('submits the revision and keeps school status out of a school administrator update',async()=>{
 mount(false,school)
 expect(screen.queryByLabelText('状态')).toBeNull()
 fireEvent.change(screen.getByLabelText('简称'),{target:{value:'示校'}})
 fireEvent.click(screen.getByRole('button',{name:'保存资料'}))
 await waitFor(()=>expect(workspace.schoolProfiles.update).toHaveBeenCalled(),{timeout:5000})
 const body=vi.mocked(workspace.schoolProfiles.update).mock.calls[0][1]
 expect(body).toMatchObject({short_name:'示校',revision:3});expect(body).not.toHaveProperty('status')
})
it('preserves local edits and disables overwrite after a revision conflict',async()=>{
 vi.mocked(workspace.schoolProfiles.update).mockRejectedValueOnce(Object.assign(new Error('修订冲突'),{status:409}))
 mount(true,school)
 fireEvent.change(screen.getByLabelText('名称'),{target:{value:'本地学校名称'}})
 fireEvent.click(screen.getByRole('button',{name:'保存资料'}))
 await screen.findByText('修订冲突',{}, {timeout:5000})
 expect(screen.getByLabelText('名称')).toHaveProperty('value','本地学校名称')
 expect(screen.getByRole('button',{name:'保存资料'})).toHaveProperty('disabled',true)
})
it('shows a saved notice after an edit succeeds',async()=>{
 mount(true,school)
 fireEvent.click(screen.getByRole('button',{name:'保存资料'}))
 expect(await screen.findByText('学校资料已保存',{}, {timeout:5000})).toBeTruthy()
})
it('uses color pickers and saves normalized theme colors',async()=>{
 mount(true,school)
 const primary=screen.getByRole('group',{name:'主题主色'})
 expect(primary.querySelector('.ant-color-picker-trigger')).toBeTruthy()
 fireEvent.click(primary.querySelector('.ant-color-picker-trigger')!)
 await waitFor(()=>expect(document.querySelectorAll('.ant-color-picker-presets-color').length).toBeGreaterThan(1))
 fireEvent.click(document.querySelectorAll('.ant-color-picker-presets-color')[1])
 fireEvent.click(screen.getByRole('button',{name:'保存资料'}))
 await waitFor(()=>expect(workspace.schoolProfiles.update).toHaveBeenCalled(),{timeout:5000})
 expect(vi.mocked(workspace.schoolProfiles.update).mock.calls[0][1]).toMatchObject({theme_primary:'#1677FF'})
})
it('closes the create drawer and shows the created school name',async()=>{
 vi.mocked(workspace.schoolProfiles.platform).mockResolvedValue({items:[],page:{number:1,size:30,has_more:false}})
 vi.mocked(workspace.schoolProfiles.create).mockResolvedValue({...school,id:'new-school',name:'新学校',revision:1})
 render(<QueryClientProvider client={new QueryClient({defaultOptions:{queries:{retry:false}}})}><Schools schools={[]} loading={false} refresh={async()=>{}} enter={()=>{}}/></QueryClientProvider>)
 fireEvent.click(screen.getByRole('button',{name:/^新\s*建$/}))
 fireEvent.change(await screen.findByLabelText('名称'),{target:{value:'新学校'}})
 fireEvent.change(screen.getByLabelText('纬度'),{target:{value:'23.1'}})
 fireEvent.change(screen.getByLabelText('经度'),{target:{value:'113.3'}})
 fireEvent.click(screen.getByRole('button',{name:'创建学校'}))
 expect(await screen.findByText(/已创建「新学校」/,{}, {timeout:5000})).toBeTruthy()
 expect(screen.queryByText('资料与图片')).toBeNull()
})
it('shows regions and coordinates in merged columns',async()=>{
 const item={...school,id:'school-1',name:'示例学校',province:'广东省',city:'广州市',area:'天河区',latitude:23.129163,longitude:113.264435}
 vi.mocked(workspace.schoolProfiles.platform).mockResolvedValue({items:[item],page:{number:1,size:30,has_more:false}})
 render(<QueryClientProvider client={new QueryClient({defaultOptions:{queries:{retry:false}}})}><Schools schools={[]} loading={false} refresh={async()=>{}} enter={()=>{}}/></QueryClientProvider>)
 expect(await screen.findByRole('columnheader',{name:'地区'})).toBeTruthy()
 expect(screen.getByRole('columnheader',{name:'坐标'})).toBeTruthy()
 expect(screen.queryByRole('columnheader',{name:'省份'})).toBeNull()
 expect(screen.queryByRole('columnheader',{name:'纬度'})).toBeNull()
 expect(await screen.findByText('广东省 / 广州市 / 天河区')).toBeTruthy()
 expect(screen.getByText('23.129163, 113.264435')).toBeTruthy()
})
it('filters schools by the selected region',async()=>{
 vi.mocked(workspace.schoolProfiles.platform).mockResolvedValue({items:[],page:{number:1,size:30,has_more:false}})
 render(<QueryClientProvider client={new QueryClient({defaultOptions:{queries:{retry:false}}})}><Schools schools={[]} loading={false} refresh={async()=>{}} enter={()=>{}}/></QueryClientProvider>)
 const filter=screen.getByLabelText('筛选省份、城市或区域')
 fireEvent.mouseDown(filter)
 fireEvent.click(filter)
 fireEvent.click(await screen.findByText('广东省'))
 await waitFor(()=>expect(workspace.schoolProfiles.platform).toHaveBeenLastCalledWith(expect.objectContaining({province:'广东省',city:'',area:''}),expect.anything()),{timeout:5000})
})

function render(ui: Parameters<typeof renderView>[0]) { return renderView(<PermissionFixture>{ui}</PermissionFixture>) }
