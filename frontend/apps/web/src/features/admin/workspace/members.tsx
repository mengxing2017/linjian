import { useRef, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Alert, Button, Checkbox, Drawer, Input, Modal, Radio, Select, Space, Table, Tag, Typography } from 'antd'
import { browserClient, createSchoolPermissionsClient, type ManagementMember, type MemberPermissions, type PermissionOverride, type School } from './api'
import { notifySuccess } from '../../../shared/notify'

const permissionsApi = createSchoolPermissionsClient(browserClient)
const roleLabel = (role: string) => ({ admin: '学校管理员', operator: '运营', member: '仅基础关系', revoked: '已撤销', unassigned: '未授权' }[role] ?? role)
const shortcuts = [
  ['帖子查看', [1]], ['帖子审核', [1, 2]], ['帖子编辑', [1, 3]], ['帖子删除', [1, 6]],
  ['评论管理', [8, 9, 10, 11]], ['举报处理', [12, 13]], ['群二维码设置', [14, 15, 16, 17]],
  ['首页广告位设置', [18, 19, 20, 21]], ['用户查看与处置', [40, 41]],
] as const

export function Members({ school, platformAdmin, currentUserId, refresh }: { school: School; platformAdmin: boolean; currentUserId?: string; refresh: () => Promise<void> }) {
  const opening = useRef(0)
  const cache = useQueryClient()
  const [dialogs, contextHolder] = Modal.useModal()
  const [search, setSearch] = useState('')
  const [page, setPage] = useState(1)
  const [roleFilter, setRoleFilter] = useState<string>()
  const [onlyOverrides, setOnlyOverrides] = useState(false)
  const [candidates, setCandidates] = useState(false)
  const [target, setTarget] = useState<MemberPermissions>()
  const [role, setRole] = useState<'admin' | 'operator'>('operator')
  const [overrides, setOverrides] = useState<PermissionOverride[]>([])
  const [reason, setReason] = useState('')
  const [error, setError] = useState('')
  const [busy, setBusy] = useState(false)
  const [conflict, setConflict] = useState(false)
  const [comparison, setComparison] = useState<MemberPermissions>()
  const scope = ['linjian', 'school-members', currentUserId, school.id]
  const catalog = useQuery({ queryKey: [...scope, 'catalog'], queryFn: ({ signal }) => permissionsApi.catalog(school.id, signal) })
  const mine = useQuery({ queryKey: ['linjian', 'school-permissions', currentUserId, school.id], queryFn: ({ signal }) => permissionsApi.mine(school.id, signal) })
  const records = useQuery({ queryKey: [...scope, candidates, page, search, roleFilter, onlyOverrides], queryFn: ({ signal }) => (candidates ? permissionsApi.candidates : permissionsApi.members)(school.id, { page, search, role: roleFilter, has_overrides: onlyOverrides ? true : undefined }, signal) })
  const editable = platformAdmin && target?.active && target.mode !== 'platform'
  const canRecommend = Boolean(currentUserId) && target?.active && !mine.isError && !platformAdmin && target?.role === 'operator' && target.id !== currentUserId && mine.data?.effective_permissions.includes('school.member.recommendation.assign') && mine.data.effective_permissions.includes('post.recommend')
  const definitions = catalog.data?.permissions ?? []
  const effective = definitions.filter(definition => definition.applicable_roles.includes(role) && (overrides.find(item => item.permission_key === definition.key)?.effect ?? (definition.default_by_role[role] ? 'allow' : 'deny')) === 'allow').map(definition => definition.key)
  const invalidDraft = overrides.filter(item => !definitions.some(definition => definition.key === item.permission_key && definition.applicable_roles.includes(role))).map(item => `目录已不支持草稿权限 ${item.permission_key}，请重新核对`)
  const missing = invalidDraft.concat(definitions.filter(definition => effective.includes(definition.key)).flatMap(definition => definition.requires.filter(key => !effective.includes(key)).map(key => `${definition.name} 缺少 ${definitions.find(item => item.key === key)?.name ?? key}`)))

  async function open(account: ManagementMember) {
    const attempt = ++opening.current
    setError('')
    try {
      const detail = await permissionsApi.detail(school.id, account.id)
      if (attempt !== opening.current) return
      setTarget(detail); setRole(detail.role === 'admin' ? 'admin' : 'operator'); setOverrides(detail.overrides); setReason(''); setConflict(false); setComparison(undefined)
    } catch (failure) { setError((failure as Error).message) }
  }
  function change(key: string, effect: string) {
    setOverrides(previous => [...previous.filter(item => item.permission_key !== key), ...(effect === 'inherit' ? [] : [{ permission_key: key, effect: effect as 'allow' | 'deny' }])])
  }
  async function save(action?: 'remove_management' | 'revoke') {
    if (!target || !catalog.data || busy || !reason.trim()) return
    const added = effective.filter(key => !target.effective_permissions.includes(key))
    const removed = target.effective_permissions.filter(key => !effective.includes(key))
    const label = (key: string) => definitions.find(definition => definition.key === key)?.name ?? key
    const confirmed = await dialogs.confirm({ okText: '确认保存', cancelText: '继续核对', title: action ? '确认取消后台职责或撤销关系？' : '确认角色与权限变更？', content: <Space orientation="vertical"><div>目标：{target.display_name} · {school.name}</div><div>角色：{roleLabel(target.role)} → {action ? roleLabel(action === 'revoke' ? 'revoked' : 'member') : roleLabel(role)}</div><div>新增：{action ? '无' : added.map(label).join('、') || '无'}</div><div>失去：{(action ? target.effective_permissions : removed).map(label).join('、') || '无'}</div><div>原因：{reason}</div><Alert type="warning" title="请核对删除、处置、发布、审核设置及授权权等高风险变化；学校可能没有可用本地管理员，平台可重新授权恢复。" /></Space> })
    if (!confirmed) return
    setBusy(true); setError('')
    try {
      const base = { catalog_version: catalog.data.catalog_version, expected_revision: target.revision, reason: reason.trim() }
      if (action) await permissionsApi.action(school.id, target.id, { ...base, action })
      else if (editable) await permissionsApi.save(school.id, target.id, { ...base, role, overrides })
      else if (canRecommend) await permissionsApi.recommendation(school.id, target.id, { ...base, effect: overrides.find(item => item.permission_key === 'post.recommend')?.effect ?? 'inherit' })
      else return
      setTarget(undefined)
      await cache.invalidateQueries({ queryKey: ['linjian'] })
      await cache.invalidateQueries({ queryKey: ['business'] })
      await refresh(); notifySuccess('成员授权已更新')
    } catch (failure) {
      setError((failure as Error).message)
      if ((failure as { status?: number }).status === 409) setConflict(true)
    } finally { setBusy(false) }
  }
  async function reloadConflict() {
    if (!target || busy) return
    setBusy(true)
    try {
      const [detail, directory] = await Promise.all([permissionsApi.detail(school.id,target.id), catalog.refetch({throwOnError:true})])
      if (detail.catalog_version !== directory.data?.catalog_version) throw new Error('权限目录仍在更新，请重新读取后核对。')
      setComparison(detail); setError('')
    } catch (failure) { setError((failure as Error).message) }
    finally { setBusy(false) }
  }
  async function applyShortcut(numbers: readonly number[]) {
    const selected = definitions.filter(definition => numbers.includes(Number(definition.id.slice(1))) && definition.applicable_roles.includes(role))
    if (await dialogs.confirm({ title: '将以下权限明确设为允许（含读取依赖）', content: selected.map(item => item.name).join('、') })) {
      setOverrides(previous => [...previous.filter(item => !selected.some(definition => definition.key === item.permission_key)), ...selected.map(item => ({ permission_key: item.key, effect: 'allow' as const }))])
    }
  }
  return <Space orientation="vertical" style={{ width: '100%' }}>
    {contextHolder}
    <Typography.Title level={4}>{school.name} · 学校成员与权限</Typography.Title>
    <Alert type="info" title="本页仅管理后台职责与学校业务权限；小程序用户请前往用户管理。" />
    {(error || records.error || catalog.error) && <Alert type="error" title={error || records.error?.message || catalog.error?.message} />}
    <Space wrap>
      <Input.Search aria-label="搜索后台成员" value={search} onChange={event => { setSearch(event.target.value); setPage(1) }} placeholder="姓名或账号" />
      <Select aria-label="角色筛选" allowClear placeholder="有效后台成员" value={roleFilter} onChange={value => { setRoleFilter(value); setPage(1) }} options={['admin', 'operator', 'member', 'revoked'].map(value => ({ value, label: roleLabel(value) }))} style={{ width: 180 }} />
      <Checkbox checked={onlyOverrides} onChange={event => { setOnlyOverrides(event.target.checked); setPage(1) }}>仅有单独配置</Checkbox>
      {platformAdmin && <Button onClick={() => { setCandidates(value => !value); setPage(1) }}>{candidates ? '返回已授权成员' : '添加后台成员'}</Button>}
    </Space>
    <Table<ManagementMember> className="linjian-data-table" tableLayout="fixed" scroll={{ x: 1000 }} rowKey="id" loading={records.isPending} dataSource={records.data?.items ?? []} pagination={false} columns={[
      { title: '姓名 / 账号', render: (_, record) => `${record.display_name} / ${record.username}` },
      { title: '角色', render: (_, record) => <Tag>{record.platform_admin ? '平台管理员（学校覆盖不可限制）' : roleLabel(record.role)}</Tag> },
      { title: '状态', render: (_, record) => record.active ? '启用' : '停用' },
      { title: '单独配置', dataIndex: 'override_count' },
      { title: '最近变更', render: (_, record) => record.updated_at ?? '—' },
      { title: '操作', fixed: 'right', width: 180, render: (_, record) => <Button onClick={() => void open(record)}>查看 / 配置权限</Button> },
    ]} />
    <Space><Button disabled={page <= 1} onClick={() => setPage(value => value - 1)}>上一页</Button><span>第 {page} 页</span><Button disabled={!records.data?.page?.has_more} onClick={() => setPage(value => value + 1)}>下一页</Button></Space>
    <Drawer title={`${target?.display_name ?? ''} · 权限配置`} size="large" open={Boolean(target)} onClose={() => { if (!busy) { opening.current++; setTarget(undefined) } }}>
      <Space orientation="vertical" style={{ width: '100%' }}>
        <div>学校：{school.name} · 授权版本：{target?.revision} · 目录：{catalog.data?.catalog_version}</div>
        {target?.mode === 'platform' && <Alert type="warning" title="平台身份不受学校覆盖限制，不能配置学校 deny。" />}
        {!editable && <Alert type="info" title={target && !target.active ? '停用账号只读，请先在后台账号管理中启用。' : canRecommend ? '通用权限只读，仅可配置本校运营推荐权限。' : '只读：通用权限仅平台可配置。'} />}
        <Radio.Group aria-label="后台角色" disabled={!editable || busy} value={role} onChange={event => { setRole(event.target.value); setOverrides([]) }} options={[{ value: 'admin', label: '学校管理员' }, { value: 'operator', label: '运营' }]} />
        {editable && <Space wrap>{shortcuts.map(([name, numbers]) => <Button key={name} size="small" onClick={() => void applyShortcut(numbers)}>{name}</Button>)}<Button onClick={async () => { if (await dialogs.confirm({ title: '清空所有覆盖，恢复角色默认？', content: '这可能重新获得先前禁止的权限；仍需保存确认。' })) setOverrides([]) }}>全部继承</Button><Button onClick={() => setOverrides(definitions.filter(item => item.applicable_roles.includes(role)).map(item => ({ permission_key: item.key, effect: 'allow' })))}>全部允许</Button><Button onClick={() => setOverrides(definitions.filter(item => item.applicable_roles.includes(role)).map(item => ({ permission_key: item.key, effect: 'deny' })))}>全部禁止</Button></Space>}
        {Array.from(new Set(definitions.map(item => item.group))).map(group => <section key={group}><Typography.Title level={5}>{group}</Typography.Title>{definitions.filter(item => item.group === group).map(definition => {
          const effect = overrides.find(item => item.permission_key === definition.key)?.effect ?? 'inherit'
          const applicable = definition.applicable_roles.includes(role)
          const disabled = busy || !applicable || (!editable && !(canRecommend && definition.key === 'post.recommend'))
          return <div key={definition.key} style={{ padding: '8px 0', borderBottom: '1px solid #eee' }}><Space wrap><strong>{definition.name}</strong>{definition.high_risk && <Tag color="orange">高风险</Tag>}<span>默认：{definition.default_by_role[role] ? '允许' : '禁止'}</span><Checkbox aria-label={`${definition.name}单独允许`} disabled={disabled} checked={effect === 'allow'} onChange={event => change(definition.key, event.target.checked ? 'allow' : 'inherit')}>单独允许</Checkbox><Checkbox aria-label={`${definition.name}单独禁止`} disabled={disabled} checked={effect === 'deny'} onChange={event => change(definition.key, event.target.checked ? 'deny' : 'inherit')}>单独禁止</Checkbox><Tag>{applicable ? effective.includes(definition.key) ? '最终允许' : '最终禁止' : '角色不适用'}</Tag></Space><div><Typography.Text type="secondary">{definition.description} · {definition.key}</Typography.Text></div>{definition.requires.length > 0 && <div>依赖：{definition.requires.map(key => definitions.find(item => item.key === key)?.name ?? key).join('、')}</div>}</div>
        })}</section>)}
        {missing.length > 0 && <Alert type="warning" title="权限依赖未满足（不会自动补权）" description={missing.join('；')} />}
        {effective.length === 0 && <Alert type="warning" title="保存后该成员没有可用后台功能，平台仍可恢复。" />}
        {(editable || canRecommend) && <><Input.TextArea aria-label="授权变更原因" value={reason} onChange={event => setReason(event.target.value)} placeholder="必填：变更原因" maxLength={500} /><Space><Button type="primary" loading={busy} disabled={!reason.trim() || missing.length > 0 || conflict} onClick={() => void save()}>预览差异并保存</Button>{editable && <><Button danger disabled={busy || !reason.trim() || conflict} onClick={() => void save('remove_management')}>取消后台职责</Button><Button danger disabled={busy || !reason.trim() || conflict} onClick={() => void save('revoke')}>撤销学校关系</Button></>}</Space></>}
        {conflict && <Alert type="warning" title="配置已被其他人修改，草稿已保留，请重新加载比较后再提交。" action={<Button loading={busy} onClick={() => void reloadConflict()}>读取最新配置</Button>} />}
        {comparison && <Alert type="info" title={`最新版本 ${comparison.revision}，角色 ${roleLabel(comparison.role)}`} description={JSON.stringify(comparison.overrides)} action={<Button onClick={() => { setTarget(comparison); setConflict(false); setComparison(undefined) }}>以最新版本继续核对草稿</Button>} />}
      </Space>
    </Drawer>
  </Space>
}
