import { useRef, useState } from 'react'
import { useQuery, useQueryClient } from '@tanstack/react-query'
import { Alert, Button, Input, Modal, Popconfirm, Space, Table } from 'antd'
import { workspace, type PostManagementFilter, type PostManagementView } from './api'
import { PaginationControls } from '../../../shared/pagination-controls'
import { postColumns } from './post-columns'
import { ReviewDetail } from './post-reviews'
import { PostEditor } from './post-editor'
import { PostImport } from './post-import'
import { PostFilters } from './post-filters'
import { PostRowActions } from './post-actions'
import { movePost } from './post-sort'
export function PostManagement({
  school,
  platformAdmin,
  context,
  publisherId,
}: {
  school: string
  platformAdmin: boolean
  context?: string
  publisherId?: string
}) {
  const [filter, setFilter] = useState<PostManagementFilter>({ page: 1 }),
    [selected, setSelected] = useState(''),
    [editor, setEditor] = useState<string>(),
    [importing, setImporting] = useState(false)
  const [error, setError] = useState(''),
    [busy, setBusy] = useState(false),
    [sorting, setSorting] = useState(false),
    [draft, setDraft] = useState<PostManagementView[]>(),
    [orderConflict, setOrderConflict] = useState(false)
  const [offline, setOffline] = useState<PostManagementView>(),
    [reason, setReason] = useState('')
  const [selectedRowId, setSelectedRowId] = useState('')
  const dragged = useRef('')
  const cache = useQueryClient()
  const q = useQuery({
    queryKey: ['linjian', 'post-management', school, filter],
    queryFn: ({ signal }) => workspace.postManagement.list(school, filter, signal),
    refetchOnWindowFocus: !sorting,
    refetchOnReconnect: !sorting,
  })
  const page = filter.page ?? 1
  const rows = draft ?? q.data?.items ?? []
  const selectedRow = rows.find((row) => row.id === selectedRowId)
  function change(patch: Partial<PostManagementFilter>) {
    setFilter((old) => ({ ...old, ...patch, page: 1 }))
    setSelected('')
    setSelectedRowId('')
    setDraft(undefined)
    setError('')
  }
  async function saved() {
    await Promise.all([
      cache.invalidateQueries({ queryKey: ['linjian', 'post-management', school] }),
      cache.invalidateQueries({
        queryKey: ['linjian', 'post-review', school],
        refetchType: 'none',
      }),
      cache.invalidateQueries({ queryKey: ['linjian', 'post-edit', school], refetchType: 'none' }),
      cache.invalidateQueries({ queryKey: ['linjian', 'source-outbox', school] }),
      cache.invalidateQueries({ queryKey: ['linjian', 'source-outbox-detail', school] }),
      cache.invalidateQueries({ queryKey: ['business'] }),
    ])
  }
  async function act(action: () => Promise<unknown>) {
    if (busy) return
    setBusy(true)
    setError('')
    try {
      await action()
      await saved()
      if (
        cache.getQueryData<{ items: PostManagementView[] }>([
          'linjian',
          'post-management',
          school,
          filter,
        ])?.items.length === 0 &&
        page > 1
      )
        setFilter((old) => ({ ...old, page: page - 1 }))
    } catch (e) {
      setError((e as Error).message)
      await q.refetch()
    } finally {
      setBusy(false)
    }
  }
  function beginSort(is_top: boolean) {
    setFilter({ page: 1, is_top })
    setDraft(undefined)
    setOrderConflict(false)
    setSorting(true)
    setSelected('')
    setSelectedRowId('')
    setError('')
  }
  async function saveOrder() {
    if (!q.data || !draft || orderConflict) return
    setBusy(true)
    setError('')
    try {
      await workspace.postManagement.reorder(school, {
        order_revision: q.data.order_revision,
        is_top: !!filter.is_top,
        page,
        page_size: 30,
        original: q.data.items.map(({ id, revision }) => ({ id, revision })),
        ordered_ids: draft.map((r) => r.id),
      })
      setDraft(undefined)
      await saved()
    } catch (e) {
      setError((e as Error).message)
      setDraft(undefined)
      setOrderConflict(true)
    } finally {
      setBusy(false)
    }
  }
  const disabled = busy || q.isFetching || !!q.error
  return (
    <Space orientation="vertical" className="full-width">
      {(error || q.error) && <Alert type="error" title={error || q.error?.message} />}
      {orderConflict && (
        <Alert type="warning" title="排序未保存，已恢复原顺序。请重新加载列表后再排序。" />
      )}
      <div className="post-management-toolbar">
        {sorting ? (
          <Space wrap>
            <p>
              当前仅排序{filter.is_top ? '置顶' : '普通'}
              帖子。可拖动行，或使用上移、下移；仅影响当前页。
            </p>
            <Button disabled={disabled || !draft || orderConflict} onClick={() => void saveOrder()}>
              保存排序
            </Button>
            <Button
              disabled={busy}
              onClick={() => {
                setDraft(undefined)
                setSorting(false)
                setOrderConflict(false)
              }}
            >
              退出排序
            </Button>
          </Space>
        ) : (
          <>
            <PostFilters filter={filter} change={change} />
            <Space wrap>
              {platformAdmin && (
                <>
                  <Button type="primary" onClick={() => setEditor('')}>
                    添加
                  </Button>
                  <Button onClick={() => setImporting(true)}>导入</Button>
                </>
              )}
              <Button
                disabled={disabled || !selectedRow}
                onClick={() => selectedRow && setEditor(selectedRow.id)}
              >
                编辑
              </Button>
              <Popconfirm
                title="确认删除选中的帖子？删除后无法恢复。"
                disabled={!selectedRow}
                onConfirm={() =>
                  selectedRow &&
                  act(() =>
                    workspace.postManagement.delete(school, selectedRow.id, selectedRow.revision),
                  )
                }
              >
                <Button danger disabled={disabled || !selectedRow}>
                  删除
                </Button>
              </Popconfirm>
              <Button disabled={disabled} onClick={() => beginSort(false)}>
                普通排序
              </Button>
              <Button disabled={disabled} onClick={() => beginSort(true)}>
                置顶排序
              </Button>
            </Space>
          </>
        )}
        <Button
          loading={q.isFetching}
          disabled={busy}
          onClick={() => {
            setDraft(undefined)
            void q.refetch().then((result) => {
              if (result.isSuccess) {
                setOrderConflict(false)
                setError('')
              }
            })
          }}
        >
          刷新
        </Button>
      </div>
      <Table<PostManagementView>
        rowSelection={
          sorting
            ? undefined
            : {
                type: 'radio',
                selectedRowKeys: selectedRowId ? [selectedRowId] : [],
                onChange: (keys) => setSelectedRowId(String(keys[0] ?? '')),
              }
        }
        rowKey="id"
        pagination={false}
        loading={q.isFetching || busy}
        dataSource={rows}
        scroll={{ x: 2100 }}
        onRow={(row, index) =>
          sorting
            ? {
                draggable: !disabled && !orderConflict,
                onDragStart: () => {
                  dragged.current = row.id
                },
                onDragOver: (e) => e.preventDefault(),
                onDrop: (e) => {
                  e.preventDefault()
                  if (!disabled && !orderConflict)
                    setDraft(movePost(rows, dragged.current, index ?? 0))
                },
              }
            : {}
        }
        columns={[
          ...postColumns(school, setSelected),
          {
            title: '操作',
            fixed: 'right',
            width: sorting ? 150 : 270,
            render: (_, row, index) =>
              sorting ? (
                <Space>
                  <Button
                    aria-label={`上移 ${row.title}`}
                    disabled={disabled || orderConflict || index === 0}
                    onClick={() => setDraft(movePost(rows, row.id, index - 1))}
                  >
                    上移
                  </Button>
                  <Button
                    aria-label={`下移 ${row.title}`}
                    disabled={disabled || orderConflict || index === rows.length - 1}
                    onClick={() => setDraft(movePost(rows, row.id, index + 1))}
                  >
                    下移
                  </Button>
                </Space>
              ) : (
                <PostRowActions
                  school={school}
                  row={row}
                  disabled={disabled}
                  review={setSelected}
                  edit={setEditor}
                  act={act}
                  offline={(row) => {
                    setOffline(row)
                    setReason('')
                  }}
                />
              ),
          },
        ]}
      />
      <PaginationControls
        page={page}
        more={q.data?.page?.has_more}
        busy={disabled || !!draft || orderConflict}
        change={(value) => {
          setSelectedRowId('')
          setFilter((old) => ({ ...old, page: value }))
          setDraft(undefined)
        }}
      />
      {selected && (
        <ReviewDetail
          management
          key={selected}
          school={school}
          id={selected}
          close={() => setSelected('')}
          saved={saved}
        />
      )}
      {editor !== undefined && (
        <PostEditor
          context={context}
          key={editor}
          school={school}
          id={editor || undefined}
          close={() => setEditor(undefined)}
          saved={saved}
        />
      )}
      {importing && (
        <PostImport
          owner={publisherId}
          context={context}
          school={school}
          close={() => setImporting(false)}
          saved={saved}
        />
      )}
      <Modal
        open={!!offline}
        title="系统下架"
        confirmLoading={busy}
        okButtonProps={{ disabled: !reason.trim() }}
        onCancel={() => setOffline(undefined)}
        onOk={() => {
          const row = offline
          if (row)
            void act(async () => {
              await workspace.postManagement.status(school, row.id, row.revision, 'offline', reason)
              setOffline(undefined)
            })
        }}
      >
        <Input.TextArea
          aria-label="下架原因"
          placeholder="请填写下架原因"
          value={reason}
          maxLength={2000}
          onChange={(e) => setReason(e.target.value)}
        />
      </Modal>
    </Space>
  )
}
