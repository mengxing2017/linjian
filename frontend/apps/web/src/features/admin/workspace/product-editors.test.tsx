import {afterEach,expect,it,vi} from 'vitest'
import {cleanup,configure,fireEvent,render,screen} from '@testing-library/react'
import {QueryClient,QueryClientProvider} from '@tanstack/react-query'
import {MemoryRouter} from 'react-router-dom'
import {ResourceEditor} from './resource-editor'
import {product,workspace} from './api'
vi.mock('./api',()=>({workspace:{list:vi.fn()},product:{templates:{preview:vi.fn(),confirm:vi.fn()},resources:{artwork:vi.fn(),preview:vi.fn(),confirm:vi.fn(),upload:vi.fn()}}}))
class Resize {observe(){}unobserve(){}disconnect(){}}
vi.stubGlobal('ResizeObserver',Resize)
Object.defineProperty(window,'matchMedia',{writable:true,value:()=>({matches:false,addListener:vi.fn(),removeListener:vi.fn(),addEventListener:vi.fn(),removeEventListener:vi.fn()})})
configure({asyncUtilTimeout:5000})
afterEach(()=>{cleanup();vi.clearAllMocks()})
function show(element:React.ReactNode){vi.mocked(workspace.list).mockResolvedValue({items:[]});vi.mocked(product.resources.artwork).mockResolvedValue({items:[]});return render(<MemoryRouter><QueryClientProvider client={new QueryClient({defaultOptions:{queries:{retry:false}}})}>{element}</QueryClientProvider></MemoryRouter>)}
it.each([
 undefined,
 {kind:'none'},
 {kind:'external_link',url:'https://example.com'},
 {kind:'internal_page',page:'feed'},
 {kind:'phone',number:'123456'},
 {kind:'copy_text',text:'联系方式'},
])('keeps resource validity dates paired for action %j',action=>{
 const item=action?{id:'resource',revision:1,kind:'service',title:'活动',content:'校园活动',image_ref:null,action,active:true,sort_order:0,starts_at:null,ends_at:null}:undefined
 show(<ResourceEditor school="school" item={item} onSaved={vi.fn()}/>)
 const starts=screen.getByLabelText('生效时间').closest('label')!
 const ends=screen.getByLabelText('失效时间').closest('label')!
 expect(starts.parentElement?.classList.contains('form-pair')).toBe(true)
 expect(ends.parentElement).toBe(starts.parentElement)
 expect(starts.parentElement?.parentElement?.classList.contains('form-grid')).toBe(true)
})
it('keeps a failed service publication preview recoverable without introducing image input',async()=>{
 const saved=vi.fn().mockResolvedValue(undefined)
 const item={id:'resource',revision:1,kind:'service',title:'活动',content:'校园活动',image_ref:null,action:{kind:'none'},active:true,sort_order:0,starts_at:null,ends_at:null}
 vi.mocked(product.resources.preview).mockResolvedValue({id:'preview',digest:'digest',expires_at:'later',summary:item as never})
 vi.mocked(product.resources.confirm).mockRejectedValue(new Error('资源已变化，请重新核对'))
 show(<ResourceEditor school="school" item={item} onSaved={saved}/>);fireEvent.click(await screen.findByRole('button',{name:/^预\s*览$/}))
 await screen.findByText(/范围：当前学校/);expect(product.resources.confirm).not.toHaveBeenCalled();expect(screen.queryByRole('img',{name:'活动'})).toBeNull()
 fireEvent.click(screen.getByRole('button',{name:'确认发布'}));await screen.findByText('资源已变化，请重新核对')
 expect(saved).not.toHaveBeenCalled();expect(screen.getByRole('button',{name:'返回编辑'})).toBeTruthy()
 expect(product.resources.preview).toHaveBeenCalledWith('school',expect.objectContaining({image_ref:null,action:{kind:'none'}}))
})
