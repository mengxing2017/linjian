import { useEffect, useState, type ReactNode } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Alert, Button } from 'antd'
import { SignIn } from './sign-in'
import { identity, subscribeUnauthorized, type User } from './api'
export function SessionGate({
  children,
  wecom = false,
  showSessionBar = true,
}: {
  children: (user: User, logout: () => Promise<void>) => ReactNode
  showSessionBar?: boolean
  wecom?: boolean
}) {
  const cache = useQueryClient()
  const q = useQuery({ queryKey: ['session'], queryFn: identity.session, retry: false })
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  useEffect(
    () =>
      subscribeUnauthorized(() => {
        // Initial login failures already render the form. Expiry must also discard
        // cached session data and protected results, including background requests.
        if (!cache.getQueryData(['session'])) return
        void cache.cancelQueries()
        cache.clear()
        cache.setQueryData(['session'], null)
        setError('登录已失效，请重新登录')
      }),
    [cache],
  )
  async function login(values: { username: string; password: string }) {
    setError('')
    setBusy(true)
    try {
      const s = await identity.login(values.username, values.password)
      await cache.cancelQueries()
      cache.clear()
      cache.setQueryData(['session'], s)
    } catch (e) {
      setError((e as Error).message)
    } finally {
      setBusy(false)
    }
  }
  async function logout() {
    setError('')
    try {
      await identity.logout()
      await cache.cancelQueries()
      cache.clear()
    } catch (e) {
      setError((e as Error).message)
    }
  }
  if (q.isPending)
    return (
      <div className="loading" role="status">
        正在确认登录状态…
      </div>
    )
  if (!q.data) return <SignIn error={error} busy={busy} wecom={wecom} onLogin={login} />
  return (
    <>
      {showSessionBar && (
        <div className="session-bar">
          <span>{q.data.user.display_name}</span>
          <Button onClick={logout}>退出登录</Button>
        </div>
      )}
      {error && <Alert type="error" showIcon className="session-error" title={<>{error}</>} />}
      {children(q.data.user, logout)}
    </>
  )
}
