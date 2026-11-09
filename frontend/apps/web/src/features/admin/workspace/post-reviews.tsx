import { ReviewImage } from './review-image'
import { useCallback, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import {
  Alert,
  Button,
  Descriptions,
  Drawer,
  Image,
  Input,
  Select,
  Space,
  Table,
  Tag,
  Tooltip,
} from 'antd'
import { workspace, type PostReview } from './api'
import { PaginationControls } from '../../../shared/pagination-controls'
import { formatDate } from '../../../shared/format-date'

const states = { pending: '待审核', approved: '已通过', rejected: '已拒绝' }

function contentPreview(content: string) {
  const chars = Array.from(content.trim().replace(/\s+/g, ' '))
  return chars.length > 120 ? chars.slice(0, 119).join('') + '…' : chars.join('')
}


function ReviewDetail({
  school,
  id,
  close,
  saved,
}: {
  school: string
  id: string
  close: () => void
  saved: () => Promise<void>
}) {
  const [reason, setReason] = useState(''),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(''),
    [conflict, setConflict] = useState(false),
    [loaded, setLoaded] = useState<Record<number, boolean>>({}),
    [imageAttempt, setImageAttempt] = useState(0)
  const report = useCallback(
    (index: number, ready: boolean) =>
      setLoaded((old) => (old[index] === ready ? old : { ...old, [index]: ready })),
    [],
  )
  const q = useQuery({
    queryKey: ['linjian', 'post-review', school, id],
    queryFn: ({ signal }) => workspace.review(school, id, signal),
    retry: false,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
  })
  const post = q.data
  const imagesReady = !!post && post.images.every((_, i) => loaded[i])
  const disabled = busy || q.isFetching || !!q.error || !post || conflict
  async function act(decision: string) {
    if (!post || disabled || (decision === 'approved' && !imagesReady)) return
    setBusy(true)
    setError('')
    try {
      await workspace.mutate(school, `posts/${id}/moderation-decisions`, {
        revision: post.revision,
        decision,
        reason,
        ...(decision === 'approved' ? { review_digest: post.review_digest } : {}),
      })
      await saved()
      close()
    } catch (e) {
      setError((e as Error).message)
      if (e && typeof e === 'object' && 'status' in e && e.status === 409) setConflict(true)
    } finally {
      setBusy(false)
    }
  }
  async function reload() {
    setLoaded({})
    setImageAttempt((value) => value + 1)
    const result = await q.refetch()
    if (result.isSuccess) {
      setConflict(false)
      setError('')
    }
  }
  async function visibility(value: string, topSort = post?.top_sort ?? 0) {
    if (!post || disabled) return
    setBusy(true)
    setError('')
    try {
      await workspace.mutate(
        school,
        `posts/${id}/visibility`,
        { revision: post.revision, visibility: value, top_sort: topSort },
        'PUT',
      )
      await saved()
      close()
    } catch (e) {
      setError((e as Error).message)
      if (e && typeof e === 'object' && 'status' in e && e.status === 409) setConflict(true)
    } finally {
      setBusy(false)
    }
  }
  return (
    <Drawer open title="内容全文审核" size="large" onClose={close}>
      {(error || q.error) && <Alert type="error" title={error || q.error?.message} />}
      {conflict && (
        <Alert type="warning" title="内容已变更，审核说明已保留。请重新加载并核对全文后再提交。" />
      )}
      {(conflict || q.error) && (
        <Button loading={q.isFetching} onClick={() => void reload()}>
          重新加载全文
        </Button>
      )}
      {q.isFetching && <p>正在读取最新全文…</p>}
      {post && (
        <>
          <Descriptions
            column={1}
            items={[
              { key: 'title', label: '标题', children: post.title },
              { key: 'school', label: '学校', children: post.school_name },
              { key: 'author', label: '用户昵称', children: post.author_nickname },
              { key: 'created', label: '创建时间', children: formatDate(post.created_at) },
              { key: 'source', label: '审核来源', children: post.decision_source === 'automatic' ? '自动审核（自动通过）' : post.decision_source === 'human' ? '人工审核' : post.decision_source === 'legacy' ? '历史记录' : '待审核' },
              { key: 'decided', label: '决定时间', children: formatDate(post.decided_at) },
              {
                key: 'status',
                label: '审核状态',
                children: <Tag>{states[post.audit_status]}</Tag>,
              },
            ]}
          />
          <h3>内容</h3>
          <p className="review-full-text">{post.content}</p>
          <Image.PreviewGroup>
            {post.images.map((source, i) => (
              <ReviewImage
                key={`${post.revision}:${imageAttempt}:${i}:${source}`}
                school={school}
                source={source}
                index={i}
                report={report}
              />
            ))}
          </Image.PreviewGroup>
          {!!post.tags.length && <p>标签：{post.tags.join('、')}</p>}
          <Input.TextArea
            aria-label="审核说明"
            value={reason}
            maxLength={2000}
            onChange={(e) => setReason(e.target.value)}
          />
          <Space wrap>
            <Button
              disabled={disabled || !imagesReady}
              loading={busy}
              type="primary"
              onClick={() => void act('approved')}
            >
              通过审核
            </Button>
            <Button disabled={disabled} danger onClick={() => void act('rejected')}>
              拒绝
            </Button>
            <Button disabled={disabled} onClick={() => void act('pending')}>
              撤销审核
            </Button>
          </Space>
          <Space wrap>
            <Button disabled={disabled} onClick={() => void visibility('visible')}>
              展示
            </Button>
            <Button disabled={disabled} onClick={() => void visibility('hidden')}>
              隐藏
            </Button>
            <Button disabled={disabled} danger onClick={() => void visibility('deleted')}>
              删除
            </Button>
            <Button
              disabled={disabled}
              onClick={() => void visibility(post.visibility, post.top_sort ? 0 : 100)}
            >
              {post.top_sort ? '取消置顶' : '置顶'}
            </Button>
          </Space>
        </>
      )}
    </Drawer>
  )
}

export function PostReviews({ school }: { school: string }) {
  const [page, setPage] = useState(1),
    [search, setSearch] = useState(''),
    [status, setStatus] = useState('pending'),
    [selected, setSelected] = useState('')
  const cache = useQueryClient()
  const query =
    '?' + new URLSearchParams({ 'page[number]': String(page), 'page[size]': '30', search, status })
  const q = useQuery({
    queryKey: ['linjian', 'post-reviews', school, query],
    queryFn: ({ signal }) => workspace.reviews(school, query, signal),
  })
  async function saved() {
    await cache.invalidateQueries({ queryKey: ['business'] })
    await cache.invalidateQueries({
      queryKey: ['linjian', 'post-review', school, selected],
      refetchType: 'none',
    })
    await cache.invalidateQueries({ queryKey: ['linjian', 'source-outbox', school] })
    await cache.invalidateQueries({ queryKey: ['linjian', 'source-outbox-detail', school] })
    await cache.invalidateQueries({ queryKey: ['linjian', 'post-reviews', school] })
    const latest = cache.getQueryData<{ items: PostReview[] }>([
      'linjian',
      'post-reviews',
      school,
      query,
    ])
    if (latest?.items.length === 0 && page > 1) setPage(page - 1)
  }
  return (
    <Space orientation="vertical" className="full-width">
      {q.error && <Alert type="error" title={q.error.message} />}
      <Space wrap>
        <Input.Search
          aria-label="搜索内容"
          placeholder="搜索标题或正文"
          onSearch={(value) => {
            setSearch(value)
            setPage(1)
            setSelected('')
          }}
        />
        <Select
          aria-label="审核状态"
          value={status}
          style={{ width: 140 }}
          onChange={(value) => {
            setStatus(value)
            setPage(1)
            setSelected('')
          }}
          options={[
            { value: '', label: '全部状态' },
            ...Object.entries(states).map(([value, label]) => ({ value, label })),
          ]}
        />
        <Button loading={q.isFetching} onClick={() => void q.refetch()}>
          刷新
        </Button>
      </Space>
      <p>列表摘要仅用于查找内容，请进入详情查看完整正文和附件后审核。</p>
      <Table<PostReview>
        rowKey="id"
        pagination={false}
        loading={q.isFetching}
        dataSource={q.data?.items ?? []}
        scroll={{ x: 1300 }}
        tableLayout="fixed"
        columns={[
          {
            title: '标题',
            width: 200,
            render: (_, row) => (
              <Button
                type="link"
                className="record-title review-title"
                title={row.title}
                onClick={() => setSelected(row.id)}
              >
                {row.title}
              </Button>
            ),
          },
          {
            title: '审核状态',
            dataIndex: 'audit_status',
            width: 110,
            render: (_, row) => (
              <Tag color={{ pending: 'warning', approved: 'success', rejected: 'error' }[row.audit_status]}>
                {states[row.audit_status]}
              </Tag>
            ),
          },
          {
            title: '内容摘要',
            width: 420,
            render: (_, row) => (
              <Tooltip title={contentPreview(row.content)} trigger={['hover', 'focus']}>
                <span className="review-list-preview" tabIndex={0}>
                  {contentPreview(row.content)}
                </span>
              </Tooltip>
            ),
          },
          { title: '学校', dataIndex: 'school_name', width: 140, ellipsis: true },
          { title: '用户昵称', dataIndex: 'author_nickname', width: 140, ellipsis: true },
          {
            title: '创建时间',
            width: 180,
            ellipsis: true,
            render: (_, row) => formatDate(row.created_at),
          },
          {
            title: '操作',
            fixed: 'right',
            width: 100,
            render: (_, row) => <Button onClick={() => setSelected(row.id)}>详情</Button>,
          },
        ]}
      />
      <PaginationControls
        page={page}
        more={q.data?.page?.has_more}
        busy={q.isFetching}
        change={(value) => {
          setPage(value)
          setSelected('')
        }}
      />
      {selected && (
        <ReviewDetail
          key={`${school}:${selected}`}
          school={school}
          id={selected}
          close={() => setSelected('')}
          saved={saved}
        />
      )}
    </Space>
  )
}
