import { FilterOutlined } from '@ant-design/icons'
import { Button, Input, Popover, Select, Space } from 'antd'
import type { PostManagementFilter } from './api'
import { postAuditLabels, postStatusLabels } from './post-columns'

export function PostFilters({
  filter,
  change,
}: {
  filter: PostManagementFilter
  change: (patch: Partial<PostManagementFilter>) => void
}) {
  const activeCount = [
    filter.audit_status,
    filter.visibility,
    filter.is_top !== undefined,
    filter.tag,
    filter.created_from,
    filter.created_to,
  ].filter(Boolean).length
  function localDate(value?: string) {
    if (!value) return ''
    const date = new Date(value)
    return new Date(date.getTime() - date.getTimezoneOffset() * 60000).toISOString().slice(0, 16)
  }
  return (
    <Space className="post-search-tools">
      <Input.Search
        aria-label="搜索"
        placeholder="标题、正文或昵称"
        value={filter.search ?? ''}
        onChange={(e) => change({ search: e.target.value })}
      />
      <Popover
        trigger="click"
        placement="bottomLeft"
        title="筛选条件"
        content={
          <>
            <div className="post-filter-fields">
              <Select
                aria-label="审核状态"
                value={filter.audit_status ?? ''}
                style={{ width: '100%' }}
                onChange={(audit_status) => change({ audit_status })}
                options={[
                  { value: '', label: '全部审核状态' },
                  ...Object.entries(postAuditLabels).map(([value, label]) => ({ value, label })),
                ]}
              />
              <Select
                aria-label="状态"
                value={filter.visibility ?? ''}
                style={{ width: '100%' }}
                onChange={(visibility) => change({ visibility })}
                options={[
                  { value: '', label: '全部状态' },
                  ...Object.entries(postStatusLabels)
                    .filter(([value]) => value !== 'deleted')
                    .map(([value, label]) => ({ value, label })),
                ]}
              />
              <Select
                aria-label="置顶状态"
                value={filter.is_top === undefined ? '' : String(filter.is_top)}
                style={{ width: '100%' }}
                onChange={(value) => change({ is_top: value === '' ? undefined : value === 'true' })}
                options={[
                  { value: '', label: '全部置顶状态' },
                  { value: 'true', label: '已置顶' },
                  { value: 'false', label: '未置顶' },
                ]}
              />
              <Input
                aria-label="标签名称"
                placeholder="标签名称"
                value={filter.tag ?? ''}
                onChange={(e) => change({ tag: e.target.value })}
              />
              <label>
                创建起始时间{' '}
                <Input
                  type="datetime-local"
                  value={localDate(filter.created_from)}
                  onChange={(e) =>
                    change({
                      created_from: e.target.value ? new Date(e.target.value).toISOString() : undefined,
                    })
                  }
                />
              </label>
              <label>
                创建结束时间{' '}
                <Input
                  type="datetime-local"
                  value={localDate(filter.created_to)}
                  onChange={(e) =>
                    change({
                      created_to: e.target.value ? new Date(e.target.value).toISOString() : undefined,
                    })
                  }
                />
              </label>
            </div>
            <Button
              type="link"
              disabled={!activeCount}
              onClick={() => change({
                audit_status: undefined,
                visibility: undefined,
                is_top: undefined,
                tag: undefined,
                created_from: undefined,
                created_to: undefined,
              })}
            >
              重置筛选
            </Button>
          </>
        }
      >
        <Button icon={<FilterOutlined />} type={activeCount ? 'primary' : 'default'}>
          筛选{activeCount ? ` (${activeCount})` : ''}
        </Button>
      </Popover>
    </Space>
  )
}
