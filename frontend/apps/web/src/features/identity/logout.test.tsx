import { type ReactNode } from 'react'
import { afterEach, expect, it, vi } from 'vitest'
import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react'
import { QueryClient, QueryClientProvider, useQuery } from '@tanstack/react-query'
import { SessionGate } from './index'
import { type User } from './api'

const user: User = {
  id: 'user', username: 'operator', display_name: '运营', platform_admin: false,
  bound_school_id: null, grants: [],
}
const headers = { 'Content-Type': 'application/vnd.api+json' }
afterEach(() => { cleanup(); vi.unstubAllGlobals() })

function mount(background?: ReactNode) {
  const cache = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity } } })
  cache.setQueryData(['session'], { csrf: 'csrf', user })
  cache.setQueryData(['private-data'], ['private school results'])
  cache.setQueryData(['site-settings', 'public'], { site_name: '林间' })
  cache.getMutationCache().build(cache, {}, {
    context: undefined, data: 'private mutation result', error: null,
    failureCount: 0, failureReason: null, isPaused: false,
    status: 'success', variables: undefined, submittedAt: 0,
  })
  render(<QueryClientProvider client={cache}><SessionGate>{(_, logout) => <>
    <p>受保护工作区</p>{background}
    <button onClick={() => { void logout() }}>测试退出入口</button>
  </>}</SessionGate></QueryClientProvider>)
  return cache
}

it('shows login after one successful logout without refetching the session', async () => {
  const fetcher = vi.fn().mockResolvedValue(Response.json({ data: null }, { headers }))
  vi.stubGlobal('fetch', fetcher)
  const cache = mount()
  fireEvent.click(screen.getByRole('button', { name: '退出登录' }))
  expect(await screen.findByRole('heading', { name: '登录工作空间' })).toBeTruthy()
  expect(screen.queryByText('受保护工作区')).toBeNull()
  expect(fetcher).toHaveBeenCalledTimes(1)
  expect(fetcher.mock.calls[0][1]?.method).toBe('DELETE')
  expect(cache.getQueryData(['session'])).toBeNull()
  expect(cache.getQueryData(['private-data'])).toBeUndefined()
  expect(cache.getMutationCache().getAll()).toHaveLength(0)
  expect(cache.getQueryData(['site-settings', 'public'])).toEqual({ site_name: '林间' })
})

it('cancels protected queries before submitting logout', async () => {
  let backgroundSignal: AbortSignal | undefined
  function Background() {
    useQuery({ queryKey: ['protected-refresh'], queryFn: ({ signal }) => {
      backgroundSignal = signal
      return new Promise(() => {})
    } })
    return null
  }
  const fetcher = vi.fn(async () => {
    expect(backgroundSignal?.aborted).toBe(true)
    return Response.json({ data: null }, { headers })
  })
  vi.stubGlobal('fetch', fetcher)
  mount(<Background />)
  fireEvent.click(screen.getByRole('button', { name: '退出登录' }))
  expect(await screen.findByRole('heading', { name: '登录工作空间' })).toBeTruthy()
})

it('deduplicates logout requests and keeps the workspace until revocation succeeds', async () => {
  let finish!: (response: Response) => void
  const fetcher = vi.fn(() => new Promise<Response>((resolve) => { finish = resolve }))
  vi.stubGlobal('fetch', fetcher)
  mount()
  const exit = screen.getByRole('button', { name: '测试退出入口' })
  fireEvent.click(exit)
  fireEvent.click(exit)
  await waitFor(() => expect(fetcher).toHaveBeenCalledTimes(1))
  expect(screen.getByText('受保护工作区')).toBeTruthy()
  expect((screen.getByRole('button', { name: /退出登录/ }) as HTMLButtonElement).disabled).toBe(true)
  finish(Response.json({ data: null }, { headers }))
  expect(await screen.findByRole('heading', { name: '登录工作空间' })).toBeTruthy()
})

it('rechecks a failed logout and allows retry when the session is still valid', async () => {
  const fetcher = vi.fn().mockRejectedValueOnce(new Error('offline'))
    .mockResolvedValueOnce(Response.json({
      data: { type: 'sessions', id: 'current', attributes: { csrf: 'csrf', user } },
    }, { headers }))
    .mockResolvedValueOnce(Response.json({ data: null }, { headers }))
  vi.stubGlobal('fetch', fetcher)
  const cache = mount()
  fireEvent.click(screen.getByRole('button', { name: '退出登录' }))
  await waitFor(() => expect(fetcher).toHaveBeenCalledTimes(2))
  await waitFor(() => expect((screen.getByRole('button', { name: /退出登录/ }) as HTMLButtonElement).disabled).toBe(false))
  expect(screen.getByRole('alert')).toBeTruthy()
  expect(screen.getByText('受保护工作区')).toBeTruthy()
  expect(cache.getQueryData(['private-data'])).toEqual(['private school results'])
  fireEvent.click(screen.getByRole('button', { name: '退出登录' }))
  expect(await screen.findByRole('heading', { name: '登录工作空间' })).toBeTruthy()
  expect(fetcher).toHaveBeenCalledTimes(3)
})

it('returns to login if the logout response was lost but the server revoked the session', async () => {
  const fetcher = vi.fn().mockRejectedValueOnce(new Error('lost response'))
    .mockResolvedValueOnce(Response.json({ errors: [{
      status: '401', code: 'unauthorized', title: 'Unauthorized', detail: '未登录',
    }] }, { status: 401, headers }))
  vi.stubGlobal('fetch', fetcher)
  const cache = mount()
  fireEvent.click(screen.getByRole('button', { name: '退出登录' }))
  expect(await screen.findByRole('heading', { name: '登录工作空间' })).toBeTruthy()
  expect(cache.getQueryData(['private-data'])).toBeUndefined()
  expect(cache.getQueryData(['session'])).toBeNull()
  expect(fetcher.mock.calls[1][0]).toBe('/api/v1/auth/session')
})
