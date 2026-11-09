import {afterEach,expect,it,vi} from 'vitest'
import {cleanup,fireEvent,render,screen,waitFor} from '@testing-library/react'
import {QueryClient,QueryClientProvider} from '@tanstack/react-query'
import {SchoolForm} from './school-profile'
import {workspace,type School} from './api'
import {schoolDefaults} from '../../../test/school'
vi.mock('./api',()=>({workspace:{schoolProfiles:{create:vi.fn(),update:vi.fn(),detail:vi.fn(),branding:vi.fn(),artwork:vi.fn()}},product:{resources:{artwork:vi.fn().mockResolvedValue([]),upload:vi.fn()}}}))
afterEach(()=>{cleanup();vi.clearAllMocks()})
const school:School={...schoolDefaults,id:'school',name:'示例学校',revision:3,config:{post_audit_mode:'manual',comment_audit_mode:'manual'}}
function mount(platform:boolean,existing?:School){render(<QueryClientProvider client={new QueryClient({defaultOptions:{queries:{retry:false}}})}><SchoolForm school={existing} platform={platform} onSaved={async()=>{}}/></QueryClientProvider>)}
it('provides the distinct coordinate and three theme fields, with images configured after creation',async()=>{
 mount(true)
 for(const label of ['学校名称','学校简称','省份','城市','区域','纬度','经度','状态','主题主色','主题辅色','主题文字色'])expect(screen.getByLabelText(label)).toBeTruthy()
 expect(screen.getByText(/先保存学校/)).toBeTruthy()
 fireEvent.change(screen.getByLabelText('学校名称'),{target:{value:'新学校'}})
 fireEvent.click(screen.getByRole('button',{name:'保存学校资料'}))
 await waitFor(()=>expect(workspace.schoolProfiles.create).toHaveBeenCalledWith(expect.objectContaining({name:'新学校',status:'normal',revision:0,theme_primary:'#6A5AF9'})),{timeout:5000})
})
it('submits the revision and keeps school status out of a school administrator update',async()=>{
 mount(false,school)
 expect(screen.queryByLabelText('状态')).toBeNull()
 fireEvent.change(screen.getByLabelText('学校简称'),{target:{value:'示校'}})
 fireEvent.click(screen.getByRole('button',{name:'保存学校资料'}))
 await waitFor(()=>expect(workspace.schoolProfiles.update).toHaveBeenCalled(),{timeout:5000})
 const body=vi.mocked(workspace.schoolProfiles.update).mock.calls[0][1]
 expect(body).toMatchObject({short_name:'示校',revision:3});expect(body).not.toHaveProperty('status')
})
it('preserves local edits and disables overwrite after a revision conflict',async()=>{
 vi.mocked(workspace.schoolProfiles.update).mockRejectedValueOnce(Object.assign(new Error('修订冲突'),{status:409}))
 mount(true,school)
 fireEvent.change(screen.getByLabelText('学校名称'),{target:{value:'本地学校名称'}})
 fireEvent.click(screen.getByRole('button',{name:'保存学校资料'}))
 await screen.findByText('修订冲突',{}, {timeout:5000})
 expect(screen.getByLabelText('学校名称')).toHaveProperty('value','本地学校名称')
 expect(screen.getByRole('button',{name:'保存学校资料'})).toHaveProperty('disabled',true)
})
