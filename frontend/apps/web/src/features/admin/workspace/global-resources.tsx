import { navigationPagination } from '../../../shared/navigation'
import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Alert, Button, Card, Drawer, Table, Tag } from 'antd'
import { workspace, type Row } from './api'
import { label } from '../../../shared/label'
import { ResourceEditor } from './resource-editor'
import { actionColumn, RowActions } from '../../../shared/row-actions'

export function GlobalResources() {
  const q = useQuery({ queryKey: ['global-resources'], queryFn: workspace.globalResources })
  const [item, setItem] = useState<Row>()
  const [open, setOpen] = useState(false)
  return (
    <section>
      <Alert
        type="info"
        showIcon
        title="学校有同类启用配置时优先使用学校配置。"
      />
      <Card
        className="data-panel"
        title="资源列表"
        extra={
          <Button
            type="primary"
            onClick={() => {
              setItem(undefined)
              setOpen(true)
            }}
          >
            新建
          </Button>
        }
      >
        <Table<Row>
          className="linjian-data-table"
          tableLayout="fixed"
          rowKey="id"
          loading={q.isFetching}
          dataSource={q.data ?? []}
          pagination={{ ...navigationPagination, pageSize: 10 }}
          scroll={{ x: 690 }}
          columns={[
            { title: '标题', width: 240, dataIndex: 'title', render: (v) => label(v) },
            { title: '类型', width: 140, dataIndex: 'kind', render: (v) => label(v) },
            {
              title: '状态',
              width: 120,
              render: (_, r) => (
                <Tag color={r.active ? 'success' : 'default'}>{r.active ? '启用' : '停用'}</Tag>
              ),
            },
            { title: '排序', width: 80, dataIndex: 'sort_order' },
            actionColumn<Row>({
              width: 110,
              render: (r) => (
                <RowActions
                  label={label(r.title) || r.id}
                  actions={[
                    {
                      key: 'edit',
                      label: '编辑',
                      onClick: () => {
                        setItem(r)
                        setOpen(true)
                      },
                    },
                  ]}
                />
              ),
            }),
          ]}
        />
      </Card>
      {open && (
        <Drawer
          open
          size="large"
          title={item ? '编辑资源' : '新建资源'}
          onClose={() => setOpen(false)}
        >
          <ResourceEditor
            key={item?.id ?? 'global'}
            item={item}
            onSaved={async () => {
              await q.refetch()
              setOpen(false)
            }}
          />
        </Drawer>
      )}
      {q.error && <Alert type="error" showIcon title={q.error.message} />}
    </section>
  )
}
