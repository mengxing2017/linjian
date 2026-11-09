import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { Alert, Button, Card, Drawer, Table, Tag } from 'antd'
import { workspace, type Row } from './api'
import { label } from '../../../shared/label'
import { ResourceEditor } from './resource-editor'

export function GlobalResources() {
  const q = useQuery({ queryKey: ['global-resources'], queryFn: workspace.globalResources })
  const [item, setItem] = useState<Row>()
  const [open, setOpen] = useState(false)
  return (
    <section>
      <Alert
        type="info"
        showIcon
        title="学校有同类启用配置时优先使用学校配置；全局屏蔽词始终生效。"
      />
      <Card
        className="data-panel"
        title="平台资源列表"
        extra={
          <Button
            type="primary"
            onClick={() => {
              setItem(undefined)
              setOpen(true)
            }}
          >
            新建平台资源
          </Button>
        }
      >
        <Table<Row>
          rowKey="id"
          loading={q.isFetching}
          dataSource={q.data ?? []}
          pagination={{ pageSize: 10 }}
          scroll={{ x: 700 }}
          columns={[
            { title: '标题 / 屏蔽词', dataIndex: 'title', render: (v) => label(v) },
            { title: '类型', dataIndex: 'kind', render: (v) => label(v) },
            {
              title: '状态',
              render: (_, r) => (
                <Tag color={r.active ? 'success' : 'default'}>{r.active ? '启用' : '停用'}</Tag>
              ),
            },
            { title: '排序', dataIndex: 'sort_order' },
            {
              title: '操作',
              render: (_, r) => (
                <Button
                  type="link"
                  onClick={() => {
                    setItem(r)
                    setOpen(true)
                  }}
                >
                  编辑
                </Button>
              ),
            },
          ]}
        />
      </Card>
      {open && (
        <Drawer
          open
          size="large"
          title={item ? '编辑平台资源' : '新建平台资源'}
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
