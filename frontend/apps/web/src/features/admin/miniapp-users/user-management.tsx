import { UserAvatar } from './user-avatar'
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Alert, Button, Descriptions, Drawer, Input, Modal, Select, Space, Table, Tag } from 'antd'
import { miniappUsers, type MiniappUser, type UserFilters } from './api'
import { formatDate } from '../../../shared/format-date'
import { PaginationControls } from '../../../shared/pagination-controls'
export function MiniappUsers({
  school,
  platformAdmin,
  schoolAdmin,
  schools,
}: {
  school?: string
  platformAdmin: boolean
  schoolAdmin: boolean
  schools: Array<{ id: string; name: string }>
}) {
  const [filters, setFilters] = useState<UserFilters>({}),
    [global, setGlobal] = useState(false),
    [selected, setSelected] = useState<string>(),
    [error, setError] = useState('')
  const [action, setAction] = useState<'status' | 'mobile' | 'school'>(),
    [reason, setReason] = useState(''),
    [value, setValue] = useState(''),
    [busy, setBusy] = useState(false)
  const scope = platformAdmin && global ? undefined : school
  const enabled = !!scope || (platformAdmin && global)
  const q = useQuery({
    queryKey: ['linjian', 'miniapp-users', scope, filters],
    queryFn: ({ signal }) => miniappUsers.list(scope, filters, signal),
    enabled,
  })
  const detail = useQuery({
    queryKey: ['linjian', 'miniapp-user', scope, selected],
    queryFn: ({ signal }) => miniappUsers.detail(scope, selected!, signal),
    enabled: enabled && !!selected,
  })
  const change = (v: Partial<UserFilters>) => {
    setSelected(undefined)
    setFilters((old) => ({ ...old, ...v, page: 1 }))
  }
  async function save() {
    if (!detail.data || !action || !reason.trim()) return
    setBusy(true)
    setError('')
    try {
      if (action === 'status')
        await miniappUsers.status(
          scope,
          detail.data,
          detail.data.status === 'normal' ? 'banned' : 'normal',
          reason.trim(),
        )
      else if (action === 'mobile')
        await miniappUsers.correctMobile(detail.data, value, reason.trim())
      else await miniappUsers.correctSchool(detail.data, value, reason.trim())
      setAction(undefined)
      await q.refetch()
      await detail.refetch()
    } catch (e) {
      setError((e as Error).message)
      await detail.refetch()
    } finally {
      setBusy(false)
    }
  }
  const titles = { status: '变更用户状态', mobile: '纠正手机号', school: '纠正所属学校' }
  return (
    <Space orientation="vertical" style={{ width: '100%' }}>
      <p>用户由微信小程序注册。封禁仅作用于小程序，不影响后台登录身份，也不会自动下架历史帖子。</p>
      {(error || q.error || detail.error) && (
        <Alert type="error" title={error || q.error?.message || detail.error?.message} />
      )}
      <Space wrap>
        {platformAdmin && (
          <Select
            aria-label="用户范围"
            value={global ? 'global' : 'school'}
            options={[
              { value: 'school', label: '当前学校' },
              { value: 'global', label: '全部学校 / 未绑定' },
            ]}
            onChange={(v) => {
              setGlobal(v === 'global')
              change({ school_id: undefined, unbound: undefined })
            }}
          />
        )}
        {platformAdmin && global && (
          <Select
            aria-label="用户所属学校"
            allowClear
            placeholder="全部学校"
            style={{ width: 190 }}
            value={filters.unbound ? 'unbound' : filters.school_id}
            options={[
              { value: 'unbound', label: '未绑定学校' },
              ...schools.map((s) => ({ value: s.id, label: s.name })),
            ]}
            onChange={(v) =>
              change({
                unbound: v === 'unbound' ? true : undefined,
                school_id: v && v !== 'unbound' ? v : undefined,
              })
            }
          />
        )}
        <Input
          aria-label="搜索用户昵称"
          placeholder="昵称"
          value={filters.search ?? ''}
          onChange={(e) => change({ search: e.target.value })}
        />
        <Input
          aria-label="搜索手机号"
          placeholder="手机号"
          value={filters.mobile ?? ''}
          maxLength={11}
          onChange={(e) => change({ mobile: e.target.value })}
        />
        <Select
          aria-label="用户状态"
          allowClear
          placeholder="全部状态"
          style={{ width: 130 }}
          value={filters.status}
          options={[
            { value: 'normal', label: '正常' },
            { value: 'banned', label: '封禁' },
          ]}
          onChange={(status) => change({ status })}
        />
        <Input
          aria-label="注册开始日期"
          type="date"
          onChange={(e) =>
            change({
              created_from: e.target.value ? e.target.value + 'T00:00:00+08:00' : undefined,
            })
          }
        />
        <Input
          aria-label="注册结束日期"
          type="date"
          onChange={(e) =>
            change({
              created_to: e.target.value ? e.target.value + 'T23:59:59.999+08:00' : undefined,
            })
          }
        />
        <Button onClick={() => void q.refetch()} disabled={!enabled}>
          刷新用户
        </Button>
      </Space>
      {!enabled ? (
        <Alert type="info" title="请选择学校，或由平台管理员切换到全部学校范围。" />
      ) : (
        <Table<MiniappUser>
          rowKey="id"
          dataSource={q.data?.items ?? []}
          loading={q.isFetching}
          pagination={false}
          scroll={{ x: 1000 }}
          onChange={(_, __, sorter) => {
            if (!Array.isArray(sorter) && sorter.field)
              change({ sort: (sorter.order === 'ascend' ? '' : '-') + String(sorter.field) })
          }}
          columns={[
            { title: '头像', dataIndex: 'avatar_url', render: (v) => <UserAvatar url={v} /> },
            { title: '昵称', dataIndex: 'nickname' },
            { title: '手机号', dataIndex: 'mobile', render: (v) => v || '未填写' },
            { title: '所属学校', dataIndex: 'school_name', render: (v) => v || '未绑定学校' },
            { title: '发帖数', dataIndex: 'post_count', sorter: true },
            {
              title: '创建时间',
              dataIndex: 'created_at',
              sorter: true,
              render: (v) => formatDate(v),
            },
            {
              title: '状态',
              dataIndex: 'status',
              render: (v) => (
                <Tag color={v === 'banned' ? 'red' : 'green'}>
                  {v === 'banned' ? '封禁' : '正常'}
                </Tag>
              ),
            },
            {
              title: '操作',
              render: (_, r) => (
                <Button
                  onClick={() => {
                    setSelected(r.id)
                    setError('')
                  }}
                >
                  详情
                </Button>
              ),
            },
          ]}
        />
      )}
      <PaginationControls
        page={filters.page ?? 1}
        more={q.data?.page?.has_more}
        busy={q.isFetching}
        change={(page) => setFilters((v) => ({ ...v, page }))}
      />
      <Drawer
        title="小程序用户详情"
        open={!!selected}
        onClose={() => {
          setSelected(undefined)
          setAction(undefined)
        }}
      >
        {detail.data && (
          <>
            <Descriptions
              column={1}
              items={[
                {
                  key: 'avatar',
                  label: '头像',
                  children: <UserAvatar url={detail.data.avatar_url} />,
                },
                ...[
                  ['用户 ID', detail.data.id],
                  ['昵称', detail.data.nickname],
                  ['手机号', detail.data.mobile || '未填写'],
                  ['所属学校', detail.data.school_name || '未绑定学校'],
                  ['发帖数', String(detail.data.post_count)],
                  ['创建时间', formatDate(detail.data.created_at)],
                  ['状态', detail.data.status === 'normal' ? '正常' : '封禁'],
                ].map(([label, children]) => ({ key: label, label, children })),
              ]}
            />
            <Space wrap>
              {(schoolAdmin || platformAdmin) && (
                <Button
                  danger={detail.data.status === 'normal'}
                  onClick={() => {
                    setAction('status')
                    setReason('')
                  }}
                >
                  {detail.data.status === 'normal' ? '封禁用户' : '解除封禁'}
                </Button>
              )}
              {platformAdmin && (
                <>
                  <Button
                    onClick={() => {
                      setAction('mobile')
                      setReason('')
                      setValue(detail.data!.mobile ?? '')
                    }}
                  >
                    纠正手机号
                  </Button>
                  <Button
                    onClick={() => {
                      setAction('school')
                      setReason('')
                      setValue(detail.data!.school_id ?? '')
                    }}
                  >
                    纠正所属学校
                  </Button>
                </>
              )}
            </Space>
            <h3>状态处置记录</h3>
            {detail.data.history?.map((record, i) => (
              <p key={i}>
                {formatDate(record.created_at)} · {record.to === 'banned' ? '封禁' : '解封'} ·{' '}
                {record.reason} · 操作者 {record.actor_id}
              </p>
            ))}
          </>
        )}
      </Drawer>
      <Modal
        title={action ? titles[action] : ''}
        open={!!action}
        onCancel={() => setAction(undefined)}
        onOk={() => void save()}
        confirmLoading={busy}
        okButtonProps={{ disabled: !reason.trim() || (action !== 'status' && !value) }}
      >
        {action === 'status' && <p>解封后须重新登录；封禁不会自动下架用户的历史内容。</p>}
        {action === 'mobile' && (
          <Input
            aria-label="新手机号"
            value={value}
            maxLength={11}
            onChange={(e) => setValue(e.target.value)}
          />
        )}
        {action === 'school' && (
          <>
            <p>不会搬迁历史内容；封禁状态保留，旧小程序会话撤销。</p>
            <Select
              aria-label="新的所属学校"
              value={value || undefined}
              style={{ width: '100%' }}
              options={schools.map((s) => ({ value: s.id, label: s.name }))}
              onChange={setValue}
            />
          </>
        )}
        <Input.TextArea
          aria-label="用户操作原因"
          placeholder="请填写原因"
          value={reason}
          maxLength={500}
          onChange={(e) => setReason(e.target.value)}
        />
      </Modal>
    </Space>
  )
}
