import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { PostRecommendation } from './post-recommendation'
import { workspace, type RecommendationPreview } from './api'
vi.mock('./api', () => ({
  workspace: {
    postManagement: {
      recommendationPreview: vi.fn(),
      recommendations: vi.fn(),
      recommend: vi.fn(),
    },
  },
}))
vi.mock('./outbox', () => ({
  Outbox: ({ initialEvent }: { initialEvent?: string }) => <p>诊断事件 {initialEvent}</p>,
}))
const preview: RecommendationPreview = {
  id: 'post',
  school_id: 'school',
  school_name: '甲学校',
  title: '优质帖子', display_label: '优质帖子',
  summary: '人工推荐摘要',
  revision: 3,
  content_revision: 1,
  publication_snapshot_id: 'snapshot',
  authorized: true,
  can_recommend: true,
  unavailable_reason: null,
  next_eligible_at: null,
  latest: null,
}
class ResizeObserver {
  observe() {}
  unobserve() {}
  disconnect() {}
}
vi.stubGlobal('ResizeObserver', ResizeObserver)
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: () => ({
    matches: false,
    addListener() {},
    removeListener() {},
    addEventListener() {},
    removeEventListener() {},
  }),
})
beforeEach(() => {
  vi.mocked(workspace.postManagement.recommendationPreview).mockResolvedValue(preview)
  vi.mocked(workspace.postManagement.recommendations).mockResolvedValue({
    items: [],
    page: { number: 1, size: 10, has_more: false },
  })
})
afterEach(() => {
  cleanup()
  vi.resetAllMocks()
})
function mount(platformAdmin = false, autoOpen = false) {
  const saved = vi.fn().mockResolvedValue(undefined)
  render(
    <QueryClientProvider
      client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}
    >
      <PostRecommendation
        school="school"
        id="post"
        saved={saved}
        platformAdmin={platformAdmin}
        autoOpen={autoOpen}
      />
    </QueryClientProvider>,
  )
  return saved
}
it('requires a human reason and retries a timeout with the exact same snapshot and request identity', async () => {
  vi.mocked(workspace.postManagement.recommend)
    .mockRejectedValueOnce(new Error('请求超时'))
    .mockResolvedValueOnce({
      id: 'event',
      school_id: 'school',
      post_id: 'post',
      recommendation_id: 'record',
      trigger_type: 'manual_recommendation',
      state: 'preparing',
      created_at: '2026-11-19T00:00:00Z',
      accepted_at: null,
      actor_name: '学校运营',
      reason: '仍有价值',
      terminated_reason: null,
    })
  const saved = mount()
  fireEvent.click(await screen.findByRole('button', { name: /^推\s*送$/ }))
  expect((screen.getByRole('button', { name: /确认推送/ }) as HTMLButtonElement).disabled).toBe(
    true,
  )
  fireEvent.change(screen.getByLabelText('再次推荐原因'), { target: { value: '仍有价值' } })
  await waitFor(() =>
    expect((screen.getByRole('button', { name: /确认推送/ }) as HTMLButtonElement).disabled).toBe(
      false,
    ),
  )
  fireEvent.click(screen.getByRole('button', { name: /确认推送/ }))
  await screen.findByText('请求超时')
  expect((screen.getByLabelText('再次推荐原因') as HTMLTextAreaElement).disabled).toBe(true)
  fireEvent.click(screen.getByRole('button', { name: /核对原请求/ }))
  await waitFor(() => expect(workspace.postManagement.recommend).toHaveBeenCalledTimes(2))
  const calls = vi.mocked(workspace.postManagement.recommend).mock.calls
  expect(calls[0]).toEqual(calls[1])
  expect(calls[0][2]).toMatchObject({
    revision: 3,
    content_revision: 1,
    publication_snapshot_id: 'snapshot',
    reason: '仍有价值',
  })
  expect(calls[0][2].idempotency_key).toBeTruthy()
  await waitFor(() => expect(saved).toHaveBeenCalled())
})
it('does not offer a push action to an operator without recommendation capability', async () => {
  vi.mocked(workspace.postManagement.recommendationPreview).mockResolvedValue({
    ...preview,
    authorized: false,
    can_recommend: false,
    unavailable_reason: 'forbidden',
  })
  mount()
  await screen.findByText('没有本校推荐权限')
  expect(screen.queryByRole('button', { name: /^推\s*送$/ })).toBeNull()
  expect(workspace.postManagement.recommend).not.toHaveBeenCalled()
})

it('opens the row action confirmation directly without creating an event', async () => {
  mount(false, true)
  await screen.findByLabelText('再次推荐原因')
  expect(workspace.postManagement.recommend).not.toHaveBeenCalled()
})
it('lets platform administrators open diagnostics for the selected source event', async () => {
  vi.mocked(workspace.postManagement.recommendations).mockResolvedValue({
    page: { number: 1, size: 10, has_more: false },
    items: [
      {
        id: 'event-selected',
        school_id: 'school',
        post_id: 'post',
        recommendation_id: null,
        trigger_type: 'review_publication',
        state: 'blocked',
        created_at: '2026-11-19T00:00:00Z',
        accepted_at: null,
        actor_name: null,
        reason: null,
        terminated_reason: null,
      },
    ],
  })
  mount(true)
  fireEvent.click(await screen.findByRole('button', { name: /查看来源诊断/ }))
  await screen.findByText('诊断事件 event-selected')
})
