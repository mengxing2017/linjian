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
function show(element:React.ReactNode){vi.mocked(workspace.list).mockResolvedValue({items:[]});vi.mocked(product.resources.artwork).mockResolvedValue([]);return render(<MemoryRouter><QueryClientProvider client={new QueryClient({defaultOptions:{queries:{retry:false}}})}>{element}</QueryClientProvider></MemoryRouter>)}
it('keeps a failed publication preview recoverable and never submits an unreviewed media URL',async()=>{
 const saved=vi.fn().mockResolvedValue(undefined)
 const item={id:'resource',revision:1,kind:'banner',title:'活动',content:'校园活动',image_ref:{kind:'asset',id:'art'},action:{kind:'none'},active:true,sort_order:0,starts_at:null,ends_at:null}
 vi.mocked(product.resources.preview).mockResolvedValue({id:'preview',digest:'digest',expires_at:'later',summary:item as never})
 vi.mocked(product.resources.confirm).mockRejectedValue(new Error('资源已变化，请重新核对'))
 show(<ResourceEditor school="school" item={item} onSaved={saved}/>);fireEvent.click(await screen.findByRole('button',{name:/^预\s*览$/}))
 await screen.findByText(/范围：当前学校/);expect(product.resources.confirm).not.toHaveBeenCalled();expect(screen.getByAltText('活动').getAttribute('src')).toContain('/resource-artwork/art')
 fireEvent.click(screen.getByRole('button',{name:'确认发布'}));await screen.findByText('资源已变化，请重新核对')
 expect(saved).not.toHaveBeenCalled();expect(screen.getByRole('button',{name:'返回编辑'})).toBeTruthy()
 expect(product.resources.preview).toHaveBeenCalledWith('school',expect.objectContaining({image_ref:{kind:'asset',id:'art'},action:{kind:'none'}}))
})
