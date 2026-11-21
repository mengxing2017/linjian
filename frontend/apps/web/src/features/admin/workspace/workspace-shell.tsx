import { Alert, Button, Empty, Modal, Select, Tag } from 'antd'
import { useQueryClient } from '@tanstack/react-query'
import { Navigate, useSearchParams } from 'react-router-dom'
import { useCallback, useRef } from 'react'
import { navigation, visibleNavigationKeys } from '../../../app/navigation'
import { AppShell } from '../../../app/shell'
import { UnsavedChangesContext } from '../../../shared/unsaved-changes'
import { useSchools } from '../../school-context'
import { StatisticsPage } from '../statistics'
import { MiniappUsers } from '../miniapp-users'
import { SourceBindings, SourceConnections } from '../source-access'
import { Accounts, Schools } from './platform'
import { OperationResources, CustomerConfiguration, OperationFeedback, OperationServices, OperationReports, SchoolMarketing } from '../operations'
import { GlobalResources } from './global-resources'
import { BlockedWords } from './blocked-words'
import { SiteSettings } from '../site-settings'
import { MiniappRegistrationSettings } from '../miniapp-registration-settings'
import { SchoolWorkspacePage } from './school-workspace-page'
import { browserClient, createSchoolPermissionsClient, type User } from './api'
import { useQuery } from '@tanstack/react-query'
import { SchoolPermissionsContext } from './permissions-context'

export function WorkspaceShell({ user, logout }: { user: User; logout: () => Promise<void> }) {
  const cache = useQueryClient()
  const q = useSchools()
  const [params, setParams] = useSearchParams()
  const dirty = useRef(false)
  const setDirty = useCallback((value: boolean) => {
    dirty.current = value
  }, [])
  const confirmLeave = () =>
    !dirty.current
      ? Promise.resolve(true)
      : new Promise<boolean>((resolve) => {
          Modal.confirm({
            title: '有未保存的修改',
            content: '离开后当前页面的未保存内容将丢失。',
            okText: '离开',
            cancelText: '继续编辑',
            onOk: () => resolve(true),
            onCancel: () => resolve(false),
          })
        })
  const guard = async (action: () => void | Promise<void>) => {
    if (await confirmLeave()) await action()
  }
  const selected = params.get('school') ?? ''
  const visibleSchools = q.data?.filter((s) => user.platform_admin || s.status === 'normal')
  const school = visibleSchools?.find((s) => s.id === (selected || visibleSchools[0]?.id))
  const role = school?.status === 'normal' ? user.grants.find((g) => g.school_id === school.id)?.role : undefined
  const authorization = useQuery({ queryKey: ['linjian', 'school-permissions', user.id, school?.id], queryFn: ({ signal }) => createSchoolPermissionsClient(browserClient).mine(school!.id, signal), enabled: Boolean(school), retry: false, staleTime: 0 })
  const effective = authorization.isError ? [] : authorization.data?.effective_permissions ?? []
  const can = (key: string) => user.platform_admin || effective.includes(key)
  const allowedKeys = visibleNavigationKeys(user.platform_admin, role, effective)
  const requestedTab = params.get('view') ?? allowedKeys[0] ?? ''
  const knownPage = navigation.find(n => n.key === requestedTab)
  const statisticsView = requestedTab === 'statistics' ? 'overview' : requestedTab.replace('statistics-', '')
  const isStatistics = knownPage?.domain === 'statistics'
  const tab = requestedTab
  const denied = !allowedKeys.includes(tab)
  const globalScope = knownPage?.scope === 'global'
  const navigate = (key: string) =>
    void guard(() =>
      setParams((previous) => {
        const next = new URLSearchParams(previous)
        next.set('view', key)
        next.delete('subview')
        return next
      }),
    )
  const enterSchool = (id: string, view: string) =>
    void guard(() => setParams({ school: id, view }))
  const refresh = async () => {
    await cache.invalidateQueries({ queryKey: ['schools'] })
    await cache.invalidateQueries({ queryKey: ['business'] })
    await cache.invalidateQueries({ queryKey: ['directory'] })
    await cache.invalidateQueries({ queryKey: ['linjian', 'school-management'] })
  }
  const legacyKind = params.get('kind') ?? params.get('subview')
  if ((requestedTab === 'resources' || requestedTab === 'platform-resources') && (legacyKind === 'banner' || legacyKind === 'qrcode')) {
    const next = new URLSearchParams(params)
    next.set('view', legacyKind === 'banner' ? 'home-banners' : 'group-qrcodes')
    next.delete('kind'); next.delete('subview')
    if (requestedTab === 'platform-resources') next.set('scope','platform')
    return <Navigate to={{search:`?${next}`}} replace />
  }
  if (requestedTab === 'cases' && legacyKind === 'feedback') {
    const next = new URLSearchParams(params)
    next.set('view','feedback'); next.delete('kind'); next.delete('subview')
    if (params.get('id')) next.set('feedback',params.get('id')!)
    return <Navigate to={{search:`?${next}`}} replace />
  }
  if (requestedTab === 'comments') {
    const next = new URLSearchParams(params)
    next.set('view', 'posts')
    next.set('subview', 'comments')
    return <Navigate to={{ search: `?${next}` }} replace />
  }
  function renderPage() {
    if (school && !user.platform_admin && authorization.isPending) return <Alert type="info" title="正在加载本校有效权限…" />
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
    if (isStatistics) return <StatisticsPage key={tab} view={statisticsView as 'overview' | 'content' | 'registration' | 'activity' | 'schools'} user={user} schools={q.data ?? []} />
    if (tab === 'customer-config' && user.platform_admin) return <CustomerConfiguration />
    if (tab === 'home-shortcuts') return <OperationServices key={school?.id ?? 'platform'} school={school?.id} />
    if (tab === 'service-targets' && user.platform_admin) return <OperationServices mode="targets" />
    if (tab === 'marketing-slots' && user.platform_admin) return <OperationServices mode="marketing" />
    if (tab === 'operation-reports') return <OperationReports key={school?.id??'platform'} school={school?.id} />
    if (tab === 'school-marketing' && school) return <SchoolMarketing key={school.id} school={school.id} />
    if (tab === 'group-qrcodes' || tab === 'home-banners') return <OperationResources key={`${school?.id}:${tab}`} module={tab} school={school?.id} platformAdmin={user.platform_admin} schoolAdmin={can('school.profile.update')} />
    if (tab === 'feedback' && school) return <OperationFeedback key={school.id} school={school.id} canDelete={can('operation.feedback.delete')} />
    if (tab === 'miniapp-users') {
      return <MiniappUsers key={school?.id ?? 'no-school'} school={school?.id}
        platformAdmin={user.platform_admin} schoolAdmin={can('school.user.status.update')} schools={q.data ?? []} />
    }
    if (tab === 'source-connections' && user.platform_admin) return <SourceConnections />
    if (tab === 'source-bindings' && school) {
      return <SourceBindings key={school.id} school={school.id} readOnly={!user.platform_admin} />
    }
    if (user.platform_admin) {
      switch (tab) {
        case 'accounts':
          return <Accounts key={tab} administrators={false} user={user} schools={q.data ?? []} />
        case 'administrators':
          return <Accounts key={tab} administrators user={user} schools={q.data ?? []} />
        case 'schools':
          return <Schools schools={q.data ?? []} loading={q.isPending} refresh={refresh} enter={enterSchool} />
        case 'platform-resources':
          return <GlobalResources />
        case 'platform-blocked-words':
          return <BlockedWords canEdit />
        case 'site-settings':
          return <SiteSettings key={tab} />
        case 'miniapp-registration-settings':
          return <MiniappRegistrationSettings key={tab} />
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
        schoolAdmin={can('school.profile.update')} refresh={refresh} />
    </div>
  }
  return (
    <SchoolPermissionsContext.Provider value={{ platform: user.platform_admin, effective }}><UnsavedChangesContext.Provider value={setDirty}>
      <AppShell
        active={tab}
        onNavigate={navigate}
        username={user.display_name}
        platformAdmin={user.platform_admin}
        allowedKeys={allowedKeys}
        schoolRole={role}
        pageExtra={
          <div className="scope-toolbar">
            {isStatistics ? <Tag>数据统计</Tag> : globalScope ? <Tag>范围：全平台</Tag> : <>
              <div>
                <span className="field-caption">当前学校</span>
                <Select
                  aria-label="当前学校"
                  className="school-select"
                  loading={q.isPending}
                  showSearch
                  optionFilterProp="label"
                  popupMatchSelectWidth={false}
                  value={school?.id}
                  options={visibleSchools?.map((s) => ({ value: s.id, label: s.name }))}
                  placeholder="选择学校"
                  onChange={(s) => void guard(async () => {
                    await cache.cancelQueries()
                    cache.removeQueries({ queryKey: ['business'] })
                    cache.removeQueries({ queryKey: ['linjian'] })
                    setParams((previous) => {
                      const next = new URLSearchParams(previous)
                      next.set('school', s)
                      return next
                    })
                  })}
                />
              </div>
            </>}
          </div>
        }
        onLogout={async () => {
          if (await confirmLeave()) await logout()
        }}
      >
        {q.error && <Alert type="error" title={q.error.message} showIcon />}
        {renderPage()}
      </AppShell>
    </UnsavedChangesContext.Provider></SchoolPermissionsContext.Provider>
  )
}
