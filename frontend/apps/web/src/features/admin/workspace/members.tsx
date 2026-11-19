import { navigationPagination } from '../../../shared/navigation'
import { ListPanel } from '../../../shared/list-panel'
import { useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  Alert,
  Button,
  Checkbox,
  Input,
  Modal,
  Popconfirm,
  Radio,
  Space,
  Switch,
  Table,
  Tag,
  type TableColumnsType,
} from 'antd'
import { identity, workspace, type Row, type School } from './api'
import { schoolRoleLabel } from '../../../app/shell'

const editableRoles = ['member', 'operator', 'admin'] as const
type EditableRole = (typeof editableRoles)[number]

const roleOptions: Array<{ value: EditableRole; label: string; description: string }> = [
  { value: 'member', label: '普通成员', description: '参与本校论坛基础功能' },
  { value: 'operator', label: '运营', description: '参与本校内容运营，可按配置使用运营功能' },
  { value: 'admin', label: '学校管理员', description: '管理本校成员、设置与内容' },
]

const memberRoleLabels: Record<string, string> = {
  unassigned: '未加入',
  member: '普通成员',
  operator: '运营',
  admin: '学校管理员',
  revoked: '已撤销',
}

const isEditableRole = (role: string): role is EditableRole =>
  (editableRoles as readonly string[]).includes(role)

const rowText = (row: Row, key: string) => String(row[key] ?? '')

function MemberRoleTag({ role }: { role: string }) {
  const color = role === 'admin' ? 'gold' : role === 'operator' ? 'blue' : undefined
  return <Tag color={color}>{memberRoleLabels[role] ?? schoolRoleLabel(role)}</Tag>
}

interface GrantTarget {
  id: string
  username: string
  displayName: string
  currentRole: string
}

export function Members({
  school,
  platformAdmin,
  currentUserId,
  refresh,
}: {
  school: School
  platformAdmin: boolean
  currentUserId?: string
  refresh: () => Promise<void>
}) {
  const cache = useQueryClient()
  const members = useQuery({
    queryKey: ['business', school.id, 'members'],
    queryFn: ({ signal }) => workspace.list(school.id, 'members', '', signal),
  })
  const accounts = useQuery({
    queryKey: ['accounts'],
    queryFn: identity.accounts,
    enabled: platformAdmin,
  })
  const [search, setSearch] = useState(''),
    [page, setPage] = useState(1)
  const [error, setError] = useState(''),
    [done, setDone] = useState('')
  const [grant, setGrant] = useState<GrantTarget | null>(null),
    [grantRole, setGrantRole] = useState<EditableRole | ''>(''),
    [confirmAdmin, setConfirmAdmin] = useState(false),
    [grantSaving, setGrantSaving] = useState(false),
    [grantError, setGrantError] = useState(''),
    [recommendSavingId, setRecommendSavingId] = useState<string | null>(null)
  const grants = new Map(members.data?.items.map((m) => [m.id, m]))
  const rows: Row[] = platformAdmin
    ? (accounts.data ?? []).map((a) => ({
        ...a,
        role: grants.get(a.id)?.role ?? 'unassigned',
        can_recommend: grants.get(a.id)?.can_recommend ?? false,
      }))
    : (members.data?.items ?? [])
  const filtered = rows.filter((a) =>
    `${a.username ?? ''} ${a.display_name ?? ''}`
      .toLocaleLowerCase()
      .includes(search.toLocaleLowerCase()),
  )
  const loading = members.isPending || (platformAdmin && accounts.isPending)
  const loadError = members.error ?? (platformAdmin ? accounts.error : null)
  const adminCount = (members.data?.items ?? []).filter(
    (member) => rowText(member, 'role') === 'admin',
  ).length
  const isOnlyAdmin = grant?.currentRole === 'admin' && adminCount <= 1
  const isAdminDemotion =
    grant?.currentRole === 'admin' && grantRole !== '' && grantRole !== 'admin'
  const isOperatorDemotion =
    grant?.currentRole === 'operator' && grantRole !== '' && grantRole !== 'operator'
  const canSave =
    Boolean(grant) &&
    grantRole !== '' &&
    grantRole !== grant?.currentRole &&
    !grantSaving &&
    (grantRole !== 'admin' || confirmAdmin)

  async function reload() {
    await Promise.all([members.refetch(), ...(platformAdmin ? [accounts.refetch()] : [])])
  }

  function openGrant(row: Row) {
    const currentRole = rowText(row, 'role') || 'unassigned'
    setGrant({
      id: row.id,
      username: rowText(row, 'username'),
      displayName: rowText(row, 'display_name') || rowText(row, 'username'),
      currentRole,
    })
    setGrantRole(isEditableRole(currentRole) ? currentRole : '')
    setConfirmAdmin(false)
    setGrantError('')
    setDone('')
  }

  function closeGrant() {
    if (grantSaving) return
    setGrant(null)
    setConfirmAdmin(false)
    setGrantError('')
  }

  async function saveGrant(role: string) {
    const target = grant
    if (!target || !role || grantSaving) return
    setGrantSaving(true)
    setGrantError('')
    setError('')
    setDone('')
    try {
      await workspace.mutate(school.id, `members/${target.id}`, { role }, 'PUT')
      setGrant(null)
      setConfirmAdmin(false)
      setDone(`已更新 ${target.displayName} 的学校角色`)
      try {
        await refresh()
        if (platformAdmin) await accounts.refetch()
        await cache.invalidateQueries({ queryKey: ['session'] })
      } catch (refreshError) {
        setError(`授权已保存，但列表刷新失败：${(refreshError as Error).message}`)
      }
    } catch (writeError) {
      setGrantError((writeError as Error).message)
    } finally {
      setGrantSaving(false)
    }
  }

  async function toggleRecommend(row: Row, enabled: boolean) {
    setRecommendSavingId(row.id)
    setError('')
    setDone('')
    try {
      await workspace.setRecommendationCapability(school.id, row.id, enabled)
      setDone(`已更新 ${rowText(row, 'display_name')} 的推荐权限`)
    } catch (toggleError) {
      setError(
        `更新 ${rowText(row, 'display_name')} 的推荐权限失败：${(toggleError as Error).message}`,
      )
    } finally {
      setRecommendSavingId(null)
      void members.refetch()
    }
  }

  const columns: TableColumnsType<Row> = [
    { title: '账号', width: 160, render: (_, a) => rowText(a, 'username') },
    { title: '昵称', width: 160, render: (_, a) => rowText(a, 'display_name') },
    ...(platformAdmin
      ? [
          {
            title: '平台身份',
            width: 140,
            render: (_: unknown, a: Row) =>
              typeof a.platform_admin === 'boolean'
                ? a.platform_admin
                  ? '平台管理员'
                  : '普通账号'
                : '—',
          },
        ]
      : []),
    {
      title: '本校角色',
      width: 150,
      render: (_, a) => <MemberRoleTag role={rowText(a, 'role') || 'unassigned'} />,
    },
    {
      title: '推荐权限',
      width: 150,
      render: (_, a) => {
        const role = rowText(a, 'role')
        if (role === 'admin') return '默认允许'
        if (role !== 'operator') return '—'
        return (
          <Switch
            aria-label={`允许 ${rowText(a, 'display_name')} 推荐帖子`}
            checked={a.can_recommend === true}
            loading={recommendSavingId === a.id}
            disabled={loading || Boolean(loadError) || recommendSavingId !== null}
            onChange={(enabled) => void toggleRecommend(a, enabled)}
          />
        )
      },
    },
    ...(platformAdmin
      ? [
          {
            title: '账号状态',
            width: 120,
            render: (_: unknown, a: Row) => (a.active ? '启用' : '停用'),
          },
        ]
      : []),
    ...(platformAdmin
      ? [
          {
            title: '操作',
            width: 120,
            fixed: 'right' as const,
            render: (_: unknown, a: Row) => (
              <Button
                type="link"
                aria-label={`授权 ${rowText(a, 'display_name')}`}
                onClick={() => openGrant(a)}
              >
                授权
              </Button>
            ),
          },
        ]
      : []),
  ]

  return (
    <ListPanel
      title="成员列表"
      filters={
        <>
          <Space wrap>
            <Input
              aria-label="搜索学校成员账号"
              placeholder="搜索账号或昵称"
              value={search}
              onChange={(e) => {
                setSearch(e.target.value)
                setPage(1)
              }}
            />
            <Button
              disabled={loading}
              loading={members.isFetching || (platformAdmin && accounts.isFetching)}
              onClick={() => void reload()}
            >
              刷新账号与授权
            </Button>
          </Space>
        </>
      }
    >
      {loadError && <Alert type="error" title={`账号或学校授权加载失败：${loadError.message}`} />}
      {error && <Alert type="error" title={error} />}
      {done && <Alert type="info" role="status" title={done} />}
      {!platformAdmin && (
        <Alert
          type="info"
          showIcon
          title="学校角色由平台管理员维护。本校学校管理员可为运营成员设置「允许推荐帖子」。"
        />
      )}
      <Table<Row>
        className="linjian-data-table"
        tableLayout="fixed"
        rowKey="id"
        loading={loading}
        dataSource={loadError ? [] : filtered}
        scroll={{ x: platformAdmin ? 1000 : 620 }}
        locale={{
          emptyText: loadError
            ? '加载失败，请刷新重试'
            : platformAdmin
              ? '没有匹配的账号，请调整搜索条件或先新建账号'
              : '本校暂无成员',
        }}
        pagination={{
          ...navigationPagination,
          current: page,
          pageSize: 10,
          showSizeChanger: false,
          onChange: setPage,
        }}
        columns={columns}
      />
      {grant && (
        <Modal
          open
          width={560}
          title={`授权 · ${grant.displayName}（${grant.username}）`}
          mask={{ closable: !grantSaving }}
          keyboard={!grantSaving}
          onCancel={closeGrant}
          footer={
            <Space>
              <Button disabled={grantSaving} onClick={closeGrant}>
                取消
              </Button>
              {isAdminDemotion ? (
                <Popconfirm
                  title="确认降级学校管理员"
                  description="降级后该账号将失去本校管理能力，请确认已有其他管理员可以继续管理本校。"
                  okText="确认保存"
                  cancelText="取消"
                  getPopupContainer={(trigger) => trigger.parentElement ?? document.body}
                  onConfirm={() => void saveGrant(grantRole)}
                >
                  <Button type="primary" loading={grantSaving} disabled={!canSave}>
                    保存授权
                  </Button>
                </Popconfirm>
              ) : (
                <Button
                  type="primary"
                  loading={grantSaving}
                  disabled={!canSave}
                  onClick={() => void saveGrant(grantRole)}
                >
                  保存授权
                </Button>
              )}
            </Space>
          }
        >
          <div className="grant-form">
            {grantError && <Alert type="error" title={grantError} />}
            <Alert
              type="info"
              showIcon
              title={`仅调整该账号在【${school.name}】内的学校角色，不影响账号启停、密码、绑定学校或其他学校授权。`}
            />
            <div>
              当前角色：<MemberRoleTag role={grant.currentRole} />
            </div>
            {grant.currentRole === 'unassigned' && (
              <p className="grant-note">该账号尚未加入本校，保存后将加入并设为所选角色。</p>
            )}
            {grant.currentRole === 'revoked' && (
              <p className="grant-note">该账号的学校授权已撤销，保存后将重新加入并设为所选角色。</p>
            )}
            {grant.id === currentUserId && (
              <Alert type="warning" showIcon title="这是当前登录账号。" />
            )}
            {isOnlyAdmin && isAdminDemotion && (
              <Alert
                type="warning"
                showIcon
                title="这是本校唯一有效学校管理员，降级后将无人管理本校，建议先授予其他账号管理员。"
              />
            )}
            {isOperatorDemotion && (
              <Alert type="warning" showIcon title="降级后该账号的独立推荐权限将自动清除。" />
            )}
            <Radio.Group
              aria-label="目标角色"
              className="grant-role-options"
              value={grantRole}
              onChange={(event) => {
                setGrantRole(event.target.value as EditableRole)
                setConfirmAdmin(false)
              }}
            >
              <Space orientation="vertical">
                {roleOptions.map((option) => (
                  <Radio key={option.value} value={option.value}>
                    <span>{option.label}</span>
                    <span className="grant-role-description">{option.description}</span>
                  </Radio>
                ))}
              </Space>
            </Radio.Group>
            {grantRole === 'admin' && (
              <Checkbox
                checked={confirmAdmin}
                onChange={(event) => setConfirmAdmin(event.target.checked)}
              >
                我确认授予该账号本校学校管理员权限
              </Checkbox>
            )}
            <p className="grant-note">
              学校管理员默认允许推荐帖子；仅运营成员可单独授予推荐权限。
            </p>
            {isEditableRole(grant.currentRole) && (
              <div className="grant-danger-zone">
                <Popconfirm
                  title="撤销学校授权"
                  description={`撤销后 ${grant.displayName} 将失去【${school.name}】的成员能力，推荐权限一并清除；不影响账号启停、密码和绑定学校。`}
                  okText="确认撤销"
                  cancelText="取消"
                  okButtonProps={{ danger: true }}
                  getPopupContainer={(trigger) => trigger.parentElement ?? document.body}
                  onConfirm={() => void saveGrant('revoked')}
                >
                  <Button danger disabled={grantSaving}>
                    撤销学校授权
                  </Button>
                </Popconfirm>
              </div>
            )}
          </div>
        </Modal>
      )}
    </ListPanel>
  )
}
