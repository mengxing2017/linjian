import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { SourceModeration } from './source-moderation'
import { workspace } from './api'
vi.mock('./api', () => ({ workspace: { sourcePreview: vi.fn(), mutate: vi.fn() } }))
const dialogs = vi.hoisted(() => ({ prompt: vi.fn() }))
vi.mock('../../../shared/use-dialogs', () => ({ useDialogs: () => ({ prompt: dialogs.prompt, contextHolder: null }) }))
afterEach(() => { cleanup(); vi.resetAllMocks() })
function mount() {
  render(<QueryClientProvider client={new QueryClient({defaultOptions:{queries:{retry:false}}})}><SourceModeration school="school-1" post={{id:'post-1',revision:2}} refresh={vi.fn().mockResolvedValue(undefined)} /></QueryClientProvider>)
}
it('submits the server preview digest with the reviewed revision', async () => {
  vi.mocked(workspace.sourcePreview).mockResolvedValue({id:'post-1',post_revision:2,content_revision:1,summary:'后端可信摘要',summary_algorithm:'plain-v1',preview_digest:'server-digest',source_projection:'miniapp_summary_v1'})
  dialogs.prompt.mockResolvedValue('已人工核对')
  mount()
  await screen.findByText('后端可信摘要')
  fireEvent.click(screen.getByRole('button',{name:'通过审核'}))
  await waitFor(() => expect(workspace.mutate).toHaveBeenCalledWith('school-1','posts/post-1/moderation-decisions',{revision:2,decision:'approved',reason:'已人工核对',source_preview_digest:'server-digest'}))
})
it('disables approval when the displayed row is older than its preview', async () => {
  vi.mocked(workspace.sourcePreview).mockResolvedValue({id:'post-1',post_revision:3,content_revision:2,summary:'已编辑后的摘要',summary_algorithm:'plain-v1',preview_digest:'new-digest',source_projection:'miniapp_summary_v1'})
  mount()
  await screen.findByText('已编辑后的摘要')
  expect((screen.getByRole('button',{name:'通过审核'}) as HTMLButtonElement).disabled).toBe(true)
  expect(workspace.mutate).not.toHaveBeenCalled()
})
