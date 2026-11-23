import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Alert, Button } from 'antd'
import { SignIn } from './sign-in'
import { identity, subscribeUnauthorized, type User } from './api'
export function SessionGate({
  children,
  showSessionBar = true,
}: {
  children: (user: User, logout: () => Promise<void>) => ReactNode
  showSessionBar?: boolean
}) {
  const cache = useQueryClient()
  const q = useQuery({ queryKey: ['session'], queryFn: identity.session, retry: false })
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const logoutRequest = useRef<Promise<void> | null>(null)
  const clearProtected = useCallback(
    () => {
      cache.removeQueries({
        predicate: (query) =>
          !(
            query.queryKey[0] === 'session' ||
            (query.queryKey[0] === 'site-settings' &&
              query.queryKey[1] === 'public')
          ),
      })
      cache.getMutationCache().clear()
    },
    [cache],
  )
  useEffect(
    () =>
      subscribeUnauthorized(() => {
        // Initial login failures already render the form. Expiry must also discard
        // cached session data and protected results, including background requests.
        if (!cache.getQueryData(['session'])) return
        void cache.cancelQueries()
        clearProtected()
        cache.setQueryData(['session'], null)
        setError('登录已失效，请重新登录')
      }),
    [cache, clearProtected],
  )
  async function login(values: { username: string; password: string }) {
    setError('')
    setBusy(true)
    try {
      const s = await identity.login(values.username, values.password)
      await cache.cancelQueries()
      clearProtected()
      cache.setQueryData(['session'], s)
    } catch (e) {
      setError((e as Error).message)
    } finally {
      setBusy(false)
    }
  }
  function logout(): Promise<void> {
    if (logoutRequest.current) return logoutRequest.current
    setError('')
    setBusy(true)
    const request = (async () => {
      try {
        await cache.cancelQueries()
        await identity.logout()
        cache.setQueryData(['session'], null)
        clearProtected()
      } catch (e) {
        setError((e as Error).message)
        const session = await identity.session().catch(() => undefined)
        if (session) cache.setQueryData(['session'], session)
      } finally {
        logoutRequest.current = null
        setBusy(false)
      }
    })()
    logoutRequest.current = request
    return request
  }
  if (q.isPending)
    return (
      <div className="loading" role="status">
        正在确认登录状态…
      </div>
    )
  if (!q.data) return <SignIn error={error} busy={busy} onLogin={login} />
  return (
    <>
      {showSessionBar && (
        <div className="session-bar">
          <span>{q.data.user.display_name}</span>
          <Button onClick={logout} loading={busy} disabled={busy}>退出登录</Button>
        </div>
      )}
      {error && <Alert type="error" showIcon className="session-error" title={<>{error}</>} />}
      {children(q.data.user, logout)}
    </>
  )
}
