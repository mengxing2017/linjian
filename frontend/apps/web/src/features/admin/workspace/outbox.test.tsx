import {afterEach,expect,it,vi} from 'vitest'
import {cleanup,fireEvent,render,screen,waitFor} from '@testing-library/react'
import {QueryClient,QueryClientProvider} from '@tanstack/react-query'
import {Outbox} from './outbox'
import {workspace} from './api'
vi.mock('./api',()=>({workspace:{list:vi.fn(),replaySource:vi.fn(),retryPreparation:vi.fn(),outboxDetail:vi.fn()}}))
afterEach(()=>{cleanup();vi.resetAllMocks()})
it('requires a recovery reason and retains event identity on retry failure',async()=>{
 vi.mocked(workspace.list).mockResolvedValue({items:[{id:'event-1',post_id:'post-1',sequence:2,event_type:'content.upsert',attempts:1,delivery_state:'blocked',recoverable:true,receipt_status:'received',receipt_summary:{sending:null},created_at:'2026-11-08T00:00:00Z'}]})
 vi.mocked(workspace.replaySource).mockRejectedValue(new Error('重试暂不可用'))
 render(<QueryClientProvider client={new QueryClient({defaultOptions:{queries:{retry:false}}})}><Outbox school="school-1"/></QueryClientProvider>)
 expect(await screen.findByText('已收件')).toBeTruthy()
 expect(screen.getByText('暂无发送结果')).toBeTruthy()
 fireEvent.click(screen.getByRole('button',{name:'重试同一事件'}))
 const schedule=screen.getByRole('button',{name:'重新调度原事件'}) as HTMLButtonElement
 expect(schedule.disabled).toBe(true)
 fireEvent.change(screen.getByLabelText('恢复原因'),{target:{value:'原凭据已修复'}})
 fireEvent.click(schedule)
 await screen.findByText('重试暂不可用')
 await waitFor(()=>expect(workspace.replaySource).toHaveBeenCalledWith('school-1','event-1','原凭据已修复'))
 expect(workspace.replaySource).toHaveBeenCalledTimes(1)
})
it('does not offer recovery for a terminal event',async()=>{
 vi.mocked(workspace.list).mockResolvedValue({items:[{id:'event-2',recoverable:false,receipt_status:'rejected'}]})
 render(<QueryClientProvider client={new QueryClient({defaultOptions:{queries:{retry:false}}})}><Outbox school="school-1"/></QueryClientProvider>)
 await screen.findByText('收件被拒绝')
 expect((screen.getByRole('button',{name:'重试同一事件'}) as HTMLButtonElement).disabled).toBe(true)
})

it('uses preparation recovery with its revision before any source delivery exists',async()=>{
 vi.mocked(workspace.list).mockResolvedValue({items:[{id:'preparing-1',delivery_state:'preparing_content',recoverable:false,preparation:{state:'blocked',revision:3,attempts:1,error_code:'permission_denied',recoverable:true}}]})
 vi.mocked(workspace.retryPreparation).mockResolvedValue(null)
 render(<QueryClientProvider client={new QueryClient({defaultOptions:{queries:{retry:false}}})}><Outbox school="school-1"/></QueryClientProvider>)
 fireEvent.click(await screen.findByRole('button',{name:'重试内容准备'}))
 fireEvent.change(screen.getByLabelText('恢复原因'),{target:{value:'权限已经开通'}})
 fireEvent.click(screen.getByRole('button',{name:'重新调度原事件'}))
 await waitFor(()=>expect(workspace.retryPreparation).toHaveBeenCalledWith('school-1','preparing-1','权限已经开通',3))
 expect(workspace.replaySource).not.toHaveBeenCalled()
})
