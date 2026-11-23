import { useEffect, useRef, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Alert, Button, Card, Input, Modal, Select, Space, Table, Tabs, Tag } from 'antd'
import { ApiError, postSeed, type SeedBatch, type SeedPreviewInput, type SeedRow } from './api'
import { LocalDateTimePicker } from '../../../shared/date-picker'
import { formatDate } from '../../../shared/format-date'
import { fromBeijingInput, toBeijingInput } from '../../../shared/beijing-time'
import { PaginationControls } from '../../../shared/pagination-controls'
import { useSchoolPermission } from './permissions-context'
import { SeedAuthorSelect } from './seed-author-select'

function blank(index: number) {
  return {
    author_key: `author-${index}`,
    title: '',
    content: '',
    tags: [],
    display_published_at: fromBeijingInput(toBeijingInput()),
  } satisfies SeedRow
}
function AvatarTile({ school, slot, author }: { school: string; slot: number; author?: string }) {
  const [url, setUrl] = useState('')
  const query = useQuery({
    queryKey: ['linjian', 'seed-avatar', school, slot, author],
    queryFn: ({ signal }) =>
      author
        ? postSeed.authorAvatar(school, author, signal)
        : postSeed.avatar(school, slot, signal),
    staleTime: Infinity,
  })
  useEffect(() => {
    if (!query.data) return
    const value = URL.createObjectURL(new Blob([query.data]))
    setUrl(value)
    return () => URL.revokeObjectURL(value)
  }, [query.data])
  return url ? (
    <img width={32} height={32} src={url} alt={`内置头像 ${slot + 1}`} />
  ) : (
    <span>头像 {slot + 1}</span>
  )
}

export function PostSeed({
  school,
  saved,
  onBlockingChange,
  onOpenPost,
}: {
  school: string
  saved: () => Promise<unknown>
  onBlockingChange: (value: boolean) => void
  onOpenPost?: (id: string) => void
}) {
  const can = useSchoolPermission()
  const cache = useQueryClient()
  const [name, setName] = useState('校园运营发帖')
  const [rows, setRows] = useState<SeedRow[]>([blank(1)])
  const [authors, setAuthors] = useState<SeedPreviewInput['authors']>([{ key: 'author-1' }])
  const [preview, setPreview] = useState<SeedBatch>()
  const [completed, setCompleted] = useState<SeedBatch>()
  const [busy, setBusy] = useState(false)
  const [uncertain, setUncertain] = useState(false)
  const [error, setError] = useState('')
  const [selected, setSelected] = useState('')
  const [tab, setTab] = useState('create')
  const [page, setPage] = useState(1)
  const [state, setState] = useState('committed')
  const [reason, setReason] = useState('')
  const [revoking, setRevoking] = useState(false)
  const nextAuthor = useRef(2)
  const commitKey = useRef('')
  const locked = busy || uncertain || !can('content.simulation.manage')
  useEffect(() => {
    onBlockingChange(busy || uncertain)
    return () => onBlockingChange(false)
  }, [busy, uncertain, onBlockingChange])
  const batches = useQuery({
    queryKey: ['linjian', 'seed-batches', school, page, state],
    queryFn: ({ signal }) => postSeed.list(school, page, state, signal),
    enabled: can('content.simulation.manage'),
  })
  const avatars = useQuery({
    queryKey: ['linjian', 'seed-avatars', school],
    queryFn: ({ signal }) => postSeed.avatars(school, signal),
    enabled: can('content.simulation.manage'),
  })
  const detail = useQuery({
    queryKey: ['linjian', 'seed-batch', school, selected],
    queryFn: ({ signal }) => postSeed.detail(school, selected, signal),
    enabled: !!selected && can('content.simulation.manage'),
  })
  const [itemPage, setItemPage] = useState(1)
  const items = useQuery({
    queryKey: ['linjian', 'seed-items', school, selected, itemPage],
    queryFn: ({ signal }) => postSeed.items(school, selected, itemPage, signal),
    enabled: !!selected && can('content.simulation.manage'),
  })
  function edit(index: number, update: Partial<SeedRow>) {
    setPreview(undefined)
    setRows((old) => old.map((row, position) => (position === index ? { ...row, ...update } : row)))
  }
  function authorEdit(key: string, update: Partial<SeedPreviewInput['authors'][number]>) {
    setPreview(undefined)
    setAuthors((old) =>
      old.map((author) => (author.key === key ? { ...author, ...update } : author)),
    )
  }
  async function act(action: () => Promise<unknown>) {
    setBusy(true)
    setError('')
    try {
      await action()
    } catch (failure) {
      setError((failure as Error).message)
    } finally {
      setBusy(false)
    }
  }
  async function refresh() {
    await Promise.all([
      cache.invalidateQueries({ queryKey: ['linjian', 'seed-batches', school] }),
      cache.invalidateQueries({ queryKey: ['linjian', 'seed-batch', school] }),
      cache.invalidateQueries({ queryKey: ['linjian', 'seed-items', school] }),
      cache.invalidateQueries({ queryKey: ['linjian', 'seed-authors', school] }),
      saved(),
    ])
  }
  return (
    <Space orientation="vertical" style={{ width: '100%' }}>
      <Alert
        type="info"
        title="仿真帖子仅在站内展示，按学校设置进行审核。发布时间用于帖子显示，不影响实际创建时间和注册统计。"
      />
      {(error || batches.error || detail.error || items.error) && (
        <Alert
          type="error"
          title={error || (batches.error ?? detail.error ?? items.error)?.message}
        />
      )}
      {uncertain && (
        <Alert
          type="warning"
          title="提交结果尚未确认，请使用原请求重试确认；确认前不能修改或关闭。"
        />
      )}
      <Tabs
        activeKey={tab}
        onChange={(value) => {
          if (!locked) setTab(value)
        }}
        items={[
          {
            key: 'create',
            label: '发布仿真帖子',
            children: (
              <Space orientation="vertical" style={{ width: '100%' }}>
                <Input
                  aria-label="批次名称"
                  maxLength={100}
                  value={name}
                  disabled={locked}
                  onChange={(event) => {
                    setName(event.target.value)
                    setPreview(undefined)
                  }}
                />
                {rows.map((row, index) => {
                  const author = authors.find((candidate) => candidate.key === row.author_key)!
                  return (
                    <Card
                      key={row.author_key}
                      title={`帖子 ${index + 1}`}
                      extra={
                        <Button
                          disabled={locked || rows.length === 1}
                          onClick={() => {
                            setPreview(undefined)
                            setRows((old) => old.filter((_, position) => position !== index))
                            setAuthors((old) =>
                              old.filter((candidate) => candidate.key !== row.author_key),
                            )
                          }}
                        >
                          移除
                        </Button>
                      }
                    >
                      <Space orientation="vertical" style={{ width: '100%' }}>
                        <Input
                          aria-label={`标题 ${index + 1}`}
                          placeholder="标题（选填）"
                          value={row.title}
                          disabled={locked}
                          onChange={(event) => edit(index, { title: event.target.value })}
                        />
                        <Input.TextArea
                          aria-label={`正文 ${index + 1}`}
                          rows={4}
                          value={row.content}
                          disabled={locked}
                          onChange={(event) => edit(index, { content: event.target.value })}
                        />
                        <Input
                          aria-label={`标签 ${index + 1}`}
                          placeholder="标签以逗号分隔"
                          value={row.tags.join(',')}
                          disabled={locked}
                          onChange={(event) =>
                            edit(index, { tags: event.target.value.split(/[,，]/).filter(Boolean) })
                          }
                        />
                        <Space wrap>
                          <span>发布时间</span>
                          <LocalDateTimePicker
                            aria-label={`发布时间 ${index + 1}`}
                            disabled={locked}
                            value={toBeijingInput(row.display_published_at)}
                            onChange={(value) =>
                              edit(index, {
                                display_published_at: fromBeijingInput(value),
                              })
                            }
                          />
                          <span>发帖作者（可输入昵称搜索）</span>
                          <SeedAuthorSelect
                            school={school}
                            label={`作者 ${index + 1}`}
                            disabled={locked}
                            value={author.existing_id}
                            change={(existing_id) => authorEdit(row.author_key, { existing_id })}
                          />
                          {!author.existing_id && (
                            <Input
                              aria-label={`昵称 ${index + 1}`}
                              placeholder="留空随机昵称"
                              maxLength={100}
                              value={author.nickname ?? ''}
                              disabled={locked}
                              onChange={(event) =>
                                authorEdit(row.author_key, { nickname: event.target.value || null })
                              }
                            />
                          )}
                        </Space>
                        {!author.existing_id && (
                          <Space wrap>
                            <Button
                              disabled={locked}
                              onClick={() => authorEdit(row.author_key, { avatar_id: null })}
                            >
                              随机头像
                            </Button>
                            {(avatars.data?.items ?? []).map(({ slot }) => (
                              <Button
                                key={slot}
                                type={author.avatar_id === slot ? 'primary' : 'default'}
                                disabled={locked}
                                aria-label={`帖子 ${index + 1} 选择头像 ${slot + 1}`}
                                onClick={() => authorEdit(row.author_key, { avatar_id: slot })}
                              >
                                <AvatarTile school={school} slot={slot} />
                              </Button>
                            ))}
                          </Space>
                        )}
                      </Space>
                    </Card>
                  )
                })}
                <Space wrap>
                  <Button
                    disabled={locked || rows.length >= 50}
                    onClick={() => {
                      const index = nextAuthor.current++
                      setPreview(undefined)
                      setRows((old) => [...old, blank(index)])
                      setAuthors((old) => [...old, { key: `author-${index}` }])
                    }}
                  >
                    添加一行
                  </Button>
                  <Button
                    disabled={locked || !name.trim()}
                    loading={busy}
                    onClick={() =>
                      void act(async () => {
                        const result = await postSeed.preview(school, { name, authors, rows })
                        setPreview(result)
                        commitKey.current = crypto.randomUUID()
                      })
                    }
                  >
                    生成预览
                  </Button>
                </Space>
                {preview && (
                  <>
                    <Alert
                      type={preview.preview_payload.errors.length ? 'error' : 'success'}
                      title={`审核方式：${preview.preview_payload.audit_mode === 'auto' ? '自动审核' : '人工待审'}；预览到期：${formatDate(preview.expires_at)}`}
                      description={preview.preview_payload.errors
                        .map((item) => `第 ${item.row} 行：${item.message}`)
                        .join('；')}
                    />
                    <Table
                      rowKey="preview_row"
                      pagination={false}
                      dataSource={preview.preview_payload.rows.map((row, index) => ({
                        ...row,
                        preview_row: index + 1,
                      }))}
                      columns={[
                        { title: '内容概览', render: (_, _row, index) => preview.row_labels[index] },
                        {
                          title: '作者',
                          render: (_, row) =>
                            preview.preview_payload.authors.find(
                              (author) => author.key === row.author_key,
                            )?.nickname,
                        },
                        {
                          title: '头像',
                          render: (_, row) => {
                            const author = preview.preview_payload.authors.find(
                              (candidate) => candidate.key === row.author_key,
                            )!
                            return (
                              <AvatarTile
                                school={school}
                                slot={author.avatar_id}
                                author={author.existing ? author.id : undefined}
                              />
                            )
                          },
                        },
                        {
                          title: '发布时间',
                          render: (_, row) => formatDate(row.display_published_at),
                        },
                      ]}
                    />
                    <Button
                      type="primary"
                      loading={busy}
                      disabled={
                        busy ||
                        !can('content.simulation.manage') ||
                        !!preview.preview_payload.errors.length
                      }
                      onClick={() =>
                        void act(async () => {
                          let result: SeedBatch
                          try {
                            result = await postSeed.commit(school, preview.id, {
                              revision: preview.revision,
                              digest: preview.preview_digest,
                              idempotency_key: commitKey.current,
                            })
                          } catch (failure) {
                            const uncertainResult =
                              !(failure instanceof ApiError) ||
                              failure.status === 0 ||
                              failure.status >= 500
                            setUncertain(uncertainResult)
                            if (!uncertainResult) setPreview(undefined)
                            throw failure
                          }
                          setUncertain(false)
                          setCompleted(result)
                          setPreview(undefined)
                          setSelected(result.id)
                          setItemPage(1)
                          setTab('batches')
                          await refresh()
                        })
                      }
                    >
                      {uncertain ? '使用原请求确认结果' : '确认提交批次'}
                    </Button>
                  </>
                )}
              </Space>
            ),
          },
          {
            key: 'batches',
            label: '批次管理',
            children: (
              <Space orientation="vertical" style={{ width: '100%' }}>
                {completed && (
                  <Alert
                    type="success"
                    title={`批次「${completed.name}」已提交`}
                    description={`共 ${completed.post_count} 篇；待审 ${completed.preview_payload.audit_mode === 'manual' ? completed.post_count : 0} 篇，通过 ${completed.preview_payload.audit_mode === 'auto' ? completed.post_count : 0} 篇。仅站内展示，不推送来源。`}
                  />
                )}
                <Select
                  aria-label="批次状态"
                  value={state}
                  disabled={locked}
                  onChange={(value) => {
                    setState(value)
                    setPage(1)
                  }}
                  options={[
                    { value: 'committed', label: '已提交' },
                    { value: 'revoked', label: '已撤销' },
                  ]}
                />
                <Table
                  rowKey="id"
                  loading={batches.isFetching}
                  pagination={false}
                  dataSource={batches.data?.items}
                  columns={[
                    { title: '批次', dataIndex: 'name' },
                    { title: '操作者', dataIndex: 'created_by', ellipsis: true },
                    {
                      title: '实际提交时间',
                      render: (_, batch) => batch.committed_at && formatDate(batch.committed_at),
                    },
                    {
                      title: '操作',
                      fixed: 'right',
                      render: (_, batch) => (
                        <Button
                          disabled={locked}
                          onClick={() => {
                            setSelected(batch.id)
                            setItemPage(1)
                          }}
                        >
                          查看
                        </Button>
                      ),
                    },
                  ]}
                />
                {(page > 1 || batches.data?.page?.has_more) && (
                  <PaginationControls
                    page={page}
                    more={batches.data?.page?.has_more}
                    busy={locked || batches.isFetching}
                    subject="批次"
                    change={setPage}
                  />
                )}
                {detail.data && (
                  <Card title={detail.data.name}>
                    <p>
                      <Tag>{detail.data.state === 'revoked' ? '已撤销' : '已提交'}</Tag>帖子{' '}
                      {detail.data.post_count} 篇；真实互动 {detail.data.interaction_count} 条
                    </p>
                    <Table
                      rowKey="id"
                      pagination={false}
                      dataSource={items.data?.items}
                      columns={[
                        {
                          title: '内容概览',
                          render: (_, item) =>
                            onOpenPost && can('content.post.read') ? (
                              <Button
                                type="link"
                                disabled={locked}
                                onClick={() => onOpenPost(item.id)}
                              >
                                {item.display_label}
                              </Button>
                            ) : (
                              item.display_label
                            ),
                        },
                        { title: '作者', dataIndex: 'author_nickname' },
                        { title: '审核状态', dataIndex: 'audit_status' },
                        { title: '展示状态', dataIndex: 'visibility' },
                        {
                          title: '发布时间',
                          render: (_, item) => formatDate(item.display_published_at),
                        },
                      ]}
                    />
                    {(itemPage > 1 || items.data?.page?.has_more) && (
                      <PaginationControls
                        page={itemPage}
                        more={items.data?.page?.has_more}
                        busy={locked || items.isFetching}
                        subject="批次帖子"
                        change={setItemPage}
                      />
                    )}
                    {detail.data.state === 'committed' && (
                      <Button
                        danger
                        disabled={locked}
                        onClick={() => {
                          setReason('')
                          setRevoking(true)
                        }}
                      >
                        撤销整批
                      </Button>
                    )}
                  </Card>
                )}
              </Space>
            ),
          },
        ]}
      />
      <Modal
        title="确认撤销仿真发帖批次"
        open={revoking}
        confirmLoading={busy}
        okButtonProps={{ disabled: !reason.trim() || !can('content.simulation.manage') }}
        onCancel={() => {
          if (!busy) setRevoking(false)
        }}
        onOk={() =>
          void act(async () => {
            if (detail.data) {
              await postSeed.revoke(school, detail.data.id, detail.data.revision, reason)
              setRevoking(false)
              await refresh()
            }
          })
        }
      >
        <p>本批次帖子将下架且不能恢复公开。真实互动、作者和审计记录保留，不影响其他批次。</p>
        <Input.TextArea
          aria-label="撤销原因"
          value={reason}
          maxLength={500}
          disabled={busy}
          onChange={(event) => setReason(event.target.value)}
        />
      </Modal>
    </Space>
  )
}
