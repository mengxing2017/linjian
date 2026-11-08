import {render,screen,fireEvent,cleanup,waitFor} from '@testing-library/react'
import {QueryClient,QueryClientProvider} from '@tanstack/react-query'
import {afterEach,expect,it,vi} from 'vitest'
import {SourceConnections,SourceBindings} from './index'
const api=vi.hoisted(()=>({connections:vi.fn(),verify:vi.fn(),bindings:vi.fn(),bind:vi.fn()}))
vi.mock('./api',()=>({sourceAccess:api}))
afterEach(()=>{cleanup();vi.resetAllMocks()})
it('shows deployment configuration and verifies without sending browser credentials',async()=>{
 const data={configured:true,origin:'https://keeper.example.com',key_id:'key-1',connection:null}
 api.connections.mockResolvedValue(data);api.verify.mockResolvedValue({id:'connection-1'})
 const cache=new QueryClient({defaultOptions:{queries:{retry:false}}});cache.setQueryData(['source-connections'],data)
 render(<QueryClientProvider client={cache}><SourceConnections/></QueryClientProvider>)
 expect(screen.getByText('https://keeper.example.com')).toBeTruthy()
 expect(screen.getByText('key-1')).toBeTruthy()
 expect(screen.getByText('已配置')).toBeTruthy()
 expect(screen.queryByRole('textbox')).toBeNull()
 expect(screen.queryByRole('button',{name:'新建连接'})).toBeNull()
 expect(api.verify).not.toHaveBeenCalled()
 fireEvent.click(screen.getByRole('button',{name:'验证连接'}))
 await waitFor(()=>expect(api.verify).toHaveBeenCalledWith())
 await waitFor(()=>expect(api.connections).toHaveBeenCalled())
})
it('shows missing deployment configuration without offering secret entry',async()=>{
 const data={configured:false,origin:null,key_id:null,connection:null}
 api.connections.mockResolvedValue(data)
 const cache=new QueryClient({defaultOptions:{queries:{retry:false}}});cache.setQueryData(['source-connections'],data)
 render(<QueryClientProvider client={cache}><SourceConnections/></QueryClientProvider>)
 expect(screen.getByText(/尚未配置 WeKeeper 连接/)).toBeTruthy()
 expect(screen.queryByRole('button',{name:'验证连接'})).toBeNull()
 expect(screen.queryByRole('textbox')).toBeNull()
})

it('edits the single school route in place and uses the global WeKeeper configuration',async()=>{
 let binding={id:'binding-1',route_id:'old-school-route',active:true,revision:1}
 api.bindings.mockImplementation(async()=>({binding}))
 api.bind.mockImplementation(async(_school,values)=>{binding={...binding,...values,revision:2};return {id:binding.id}})
 const cache=new QueryClient({defaultOptions:{queries:{retry:false}}});cache.setQueryData(['source-bindings','school-1'],{binding})
 render(<QueryClientProvider client={cache}><SourceBindings school="school-1"/></QueryClientProvider>)
 const input=await screen.findByLabelText('消息路由 ID（route_id）') as HTMLInputElement
 await waitFor(()=>expect(input.value).toBe('old-school-route'))
 expect(screen.queryByRole('button',{name:'新增路由'})).toBeNull()
 expect(screen.queryByRole('combobox')).toBeNull()
 expect(screen.queryByLabelText('WeKeeper 连接')).toBeNull()
 expect(api.connections).not.toHaveBeenCalled()
 fireEvent.change(input,{target:{value:'replacement-school-route'}})
 const save=screen.getByRole('button',{name:'保存学校路由'}) as HTMLButtonElement
 await waitFor(()=>expect(save.disabled).toBe(false))
 fireEvent.click(save)
 await waitFor(()=>expect(api.bind).toHaveBeenCalledWith('school-1',{route_id:'replacement-school-route',active:true}))
 await waitFor(()=>expect(cache.getQueryData(['source-bindings','school-1'])).toEqual({binding:{...binding,id:'binding-1'}}))
})
it('configures one route for an unbound school without a WeKeeper connection selector',async()=>{
 api.bindings.mockResolvedValue({binding:null});api.bind.mockResolvedValue({id:'binding-2'})
 const cache=new QueryClient({defaultOptions:{queries:{retry:false}}});cache.setQueryData(['source-bindings','school-2'],{binding:null})
 render(<QueryClientProvider client={cache}><SourceBindings school="school-2"/></QueryClientProvider>)
 const input=screen.getByLabelText('消息路由 ID（route_id）') as HTMLInputElement
 expect(input.value).toBe('')
 expect(screen.queryByRole('combobox')).toBeNull()
 expect(api.connections).not.toHaveBeenCalled()
 fireEvent.change(input,{target:{value:'school-two-route'}})
 const save=screen.getByRole('button',{name:'保存学校路由'}) as HTMLButtonElement
 await waitFor(()=>expect(save.disabled).toBe(false))
 fireEvent.click(save)
 await waitFor(()=>expect(api.bind).toHaveBeenCalledWith('school-2',{route_id:'school-two-route',active:true}))
})
