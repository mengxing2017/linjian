import { LocalDateTimePicker } from '../../../shared/date-picker'
import { fromBeijingInput, toBeijingInput } from '../../../shared/beijing-time'
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
    filter.created_via,
    filter.author_kind,
    filter.seed_batch_id,
    filter.display_from,
    filter.display_to,
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
                aria-label="投稿来源"
                value={filter.created_via ?? ''}
                onChange={(created_via) => change({ created_via })}
                options={[
                  { value: '', label: '全部来源' },
                  { value: 'miniapp', label: '小程序投稿' },
                  { value: 'web', label: '后台直接发布' },
                  { value: 'seed', label: '运营仿真' },
                ]}
              />
              <Select
                aria-label="作者类型"
                value={filter.author_kind ?? ''}
                onChange={(author_kind) => change({ author_kind })}
                options={[
                  { value: '', label: '全部作者' },
                  { value: 'ordinary', label: '普通账号' },
                  { value: 'simulated', label: '模拟作者' },
                ]}
              />
              <Input.Search
                key={filter.seed_batch_id ?? ''}
                aria-label="批次 ID"
                placeholder="批次 ID（回车筛选）"
                defaultValue={filter.seed_batch_id ?? ''}
                onSearch={(value) => change({ seed_batch_id: value || undefined })}
              />
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
                onChange={(value) =>
                  change({ is_top: value === '' ? undefined : value === 'true' })
                }
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
                <LocalDateTimePicker
                  value={localDate(filter.created_from)}
                  onChange={(value) =>
                    change({
                      created_from: value ? new Date(value).toISOString() : undefined,
                    })
                  }
                />
              </label>
              <label>
                创建结束时间{' '}
                <LocalDateTimePicker
                  value={localDate(filter.created_to)}
                  onChange={(value) =>
                    change({
                      created_to: value ? new Date(value).toISOString() : undefined,
                    })
                  }
                />
              </label>
              <label>
                发布时间（起）
                <LocalDateTimePicker
                  value={filter.display_from ? toBeijingInput(filter.display_from) : ''}
                  onChange={(value) =>
                    change({
                      display_from: fromBeijingInput(value) || undefined,
                    })
                  }
                />
              </label>
              <label>
                发布时间（止）
                <LocalDateTimePicker
                  value={filter.display_to ? toBeijingInput(filter.display_to) : ''}
                  onChange={(value) =>
                    change({
                      display_to: fromBeijingInput(value) || undefined,
                    })
                  }
                />
              </label>
            </div>
            <Button
              type="link"
              disabled={!activeCount}
              onClick={() =>
                change({
                  audit_status: undefined,
                  visibility: undefined,
                  is_top: undefined,
                  tag: undefined,
                  created_via: undefined,
                  author_kind: undefined,
                  seed_batch_id: undefined,
                  display_from: undefined,
                  display_to: undefined,
                  created_from: undefined,
                  created_to: undefined,
                })
              }
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
