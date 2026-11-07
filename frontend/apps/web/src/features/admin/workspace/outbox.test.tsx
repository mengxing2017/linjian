import {afterEach,expect,it,vi} from 'vitest'
import {cleanup,fireEvent,render,screen,waitFor} from '@testing-library/react'
import {QueryClient,QueryClientProvider} from '@tanstack/react-query'
import {Outbox} from './outbox'
import {workspace} from './api'
vi.mock('./api',()=>({workspace:{list:vi.fn(),mutate:vi.fn()}}))
afterEach(()=>{cleanup();vi.resetAllMocks()})
it('distinguishes applied content from sending and preserves the original event on retry failure',async()=>{
 vi.mocked(workspace.list).mockResolvedValue({items:[{id:'event-1',post_id:'post-1',sequence:2,event_type:'content.upsert',attempts:1,receipt_status:'applied',receipt_summary:{status:'applied',sending:null},created_at:'2026-11-08T00:00:00Z'}]})
 vi.mocked(workspace.mutate).mockRejectedValue(new Error('重试暂不可用'))
 render(<QueryClientProvider client={new QueryClient({defaultOptions:{queries:{retry:false}}})}><Outbox school="school-1"/></QueryClientProvider>)
 expect(await screen.findByText('素材已更新')).toBeTruthy()
 expect(screen.getByText('暂无发送结果')).toBeTruthy()
 fireEvent.click(screen.getByRole('button',{name:'重试同一事件'}))
 await screen.findByText('重试暂不可用')
 await waitFor(()=>expect(workspace.mutate).toHaveBeenCalledWith('school-1','source-outbox/event-1/replays'))
 expect(screen.getByText('event-1')).toBeTruthy()
 expect(workspace.mutate).toHaveBeenCalledTimes(1)
})
