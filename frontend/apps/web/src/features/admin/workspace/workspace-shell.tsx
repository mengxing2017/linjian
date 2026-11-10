import { Alert, Button, Empty, Select, Tag } from 'antd'
import { useQueryClient } from '@tanstack/react-query'
import { Navigate, useSearchParams } from 'react-router-dom'
import { navigation, visibleNavigationKeys } from '../../../app/navigation'
import { AppShell } from '../../../app/shell'
import { useSchools } from '../../school-context'
import { MiniappUsers } from '../miniapp-users'
import { SourceBindings, SourceConnections } from '../source-access'
import { Directory } from './directory'
import { Accounts, Schools } from './platform'
import { GlobalResources } from './global-resources'
import { BlockedWords } from './blocked-words'
import { SchoolWorkspacePage } from './school-workspace-page'
import type { User } from './api'

export function WorkspaceShell({ user, logout }: { user: User; logout: () => Promise<void> }) {
  const cache = useQueryClient()
  const q = useSchools()
  const [params, setParams] = useSearchParams()
  const selected = params.get('school') ?? ''
  const visibleSchools = q.data?.filter((s) => user.platform_admin || s.status === 'normal')
  const school = visibleSchools?.find((s) => s.id === (selected || visibleSchools[0]?.id))
  const role = school?.status === 'normal' ? user.grants.find((g) => g.school_id === school.id)?.role : undefined
  const allowedKeys = visibleNavigationKeys(user.platform_admin, role)
  const requestedTab = params.get('view') ?? allowedKeys[0] ?? ''
  const knownPage = navigation.find(n => n.key === requestedTab)
  const tab = requestedTab
  const denied = !allowedKeys.includes(tab)
  const globalScope = knownPage?.scope === 'global' || (knownPage?.scope === 'selectable' && user.platform_admin && params.get('scope') !== 'school')
  const navigate = (key: string) =>
    setParams((previous) => {
      const next = new URLSearchParams(previous)
      next.set('view', key)
      next.delete('subview')
      return next
    })
  const enterSchool = (id: string, view: string) => setParams({ school: id, view })
  const refresh = async () => {
    await cache.invalidateQueries({ queryKey: ['schools'] })
    await cache.invalidateQueries({ queryKey: ['business'] })
    await cache.invalidateQueries({ queryKey: ['directory'] })
    await cache.invalidateQueries({ queryKey: ['linjian', 'school-management'] })
  }
  if (requestedTab === 'comments') {
    const next = new URLSearchParams(params)
    next.set('view', 'posts')
    next.set('subview', 'comments')
    return <Navigate to={{ search: `?${next}` }} replace />
  }
  function renderPage() {
    const fallback = () => navigate(allowedKeys[0] ?? 'posts')
    if (!tab) {
      return q.isPending
        ? <Empty className="empty-state" description="正在加载学校…" />
        : <Alert type="info" title="当前账号没有该学校的后台管理权限，请联系平台管理员。" />
    }
    if (!knownPage) {
      return <Alert type="warning" title="页面不存在" description={<Button onClick={fallback}>返回可用功能</Button>} />
    }
    if (denied) {
      return <Alert type="warning" title="当前账号没有此功能的访问权限" description={
        <>请核对当前学校与角色，或从导航选择已授权功能。<Button onClick={fallback}>返回可用功能</Button></>
      } />
    }
    if (tab === 'miniapp-users') {
      return <MiniappUsers key={school?.id ?? 'no-school'} school={school?.id}
        platformAdmin={user.platform_admin} schoolAdmin={role === 'admin'} schools={q.data ?? []} />
    }
    if (tab === 'source-connections' && user.platform_admin) return <SourceConnections />
    if (tab === 'source-bindings' && school) {
      return <SourceBindings key={school.id} school={school.id} readOnly={!user.platform_admin} />
    }
    if (user.platform_admin) {
      switch (tab) {
        case 'accounts':
          if (params.get('subview') === 'directory') return <Directory />
          return <Accounts key={tab} administrators={false} user={user} schools={q.data ?? []} />
        case 'administrators':
          return <Accounts key={tab} administrators user={user} schools={q.data ?? []} />
        case 'schools':
          return <Schools schools={q.data ?? []} loading={q.isPending} refresh={refresh} enter={enterSchool} />
        case 'platform-resources':
          return <GlobalResources />
        case 'platform-blocked-words':
          return <BlockedWords canEdit />
      }
    }
    if (q.isPending) return <Empty className="empty-state" description="正在加载学校…" />
    if (!school) {
      return <Empty className="empty-state" description={<>
        <h2>尚未选择学校</h2>
        <p>{user.platform_admin
          ? '请在「学校管理」中创建学校，或选择已有学校。'
          : '请联系管理员为账号分配学校权限。'}</p>
        {user.platform_admin && <Button type="primary" onClick={() => navigate('schools')}>创建学校</Button>}
      </>} />
    }
    return <div className="business-panel" key={`${school.id}-${tab}`}>
      <SchoolWorkspacePage publisherId={user.id} publisher={user.display_name} view={tab} school={school} platformAdmin={user.platform_admin}
        schoolAdmin={user.platform_admin || role === 'admin'} refresh={refresh} />
    </div>
  }
  return (
    <AppShell
      active={tab}
      onNavigate={navigate}
      username={user.display_name}
      platformAdmin={user.platform_admin}
      allowedKeys={allowedKeys}
      schoolRole={role}
      pageExtra={
        <div className="scope-toolbar">
          {globalScope ? <Tag>范围：全平台</Tag> : <>
            <div>
              <span className="field-caption">当前学校</span>
              <Select
                aria-label="当前学校"
                className="school-select"
                loading={q.isPending}
                value={school?.id}
                options={visibleSchools?.map((s) => ({ value: s.id, label: s.name }))}
                placeholder="选择学校"
                onChange={async (s) => {
                  await cache.cancelQueries()
                  cache.removeQueries({ queryKey: ['business'] })
                  cache.removeQueries({ queryKey: ['linjian'] })
                  setParams((previous) => {
                    const next = new URLSearchParams(previous)
                    next.set('school', s)
                    return next
                  })
                }}
              />
            </div>
          </>}
        </div>
      }
      onLogout={logout}
    >
      {q.error && <Alert type="error" title={q.error.message} showIcon />}
      {renderPage()}
    </AppShell>
  )
}
