import { useState } from 'react'
import { Button, Dropdown, Image, Tag, Tooltip } from 'antd'
import { DownOutlined, PushpinFilled, PushpinOutlined } from '@ant-design/icons'
import type { TableColumnsType } from 'antd'
import type { PostManagementView } from './api'
import { ReviewImage } from './review-image'
import { formatDate } from '../../../shared/format-date'
export const postAuditLabels = { pending: '待审核', approved: '已通过', rejected: '已驳回' }
export const postStatusLabels = {
  visible: '正常',
  hidden: '隐藏',
  offline: '系统下架',
  deleted: '已删除',
}
export function postColumns(
  school: string,
  detail: (id: string) => void,
  pin: { toggle: (row: PostManagementView) => void; disabled: boolean },
  status: {
    change: (row: PostManagementView, visibility: 'visible' | 'hidden' | 'offline') => void
    disabled: boolean
  },
): TableColumnsType<PostManagementView> {
  return [
    {
      title: '内容概览',
      fixed: 'left',
      width: 220,
      render: (_, r) => (
        <div className="post-title-with-pin">
          <Button type="link" className="record-title review-title" onClick={() => detail(r.id)}>
            {r.display_label}
          </Button>
          <Tooltip title={r.is_top ? '取消置顶' : '置顶'}>
            <Button
              type="text"
              size="small"
              className="post-pin"
              aria-label={`${r.is_top ? '取消置顶' : '置顶'} ${r.display_label}`}
              aria-pressed={r.is_top}
              disabled={pin.disabled}
              icon={r.is_top ? <PushpinFilled /> : <PushpinOutlined />}
              onClick={() => pin.toggle(r)}
            />
          </Tooltip>
        </div>
      ),
    },
    { title: '投稿来源', width: 140, render: (_, row) => ({ miniapp: '小程序投稿', web: '后台直接发布', seed: '运营仿真', legacy: '历史/未知' })[row.created_via] },
    { title: '模拟批次', dataIndex: 'seed_batch_id', width: 180, ellipsis: true },
    { title: '创建操作者', dataIndex: 'created_by', width: 180, ellipsis: true },
    { title: '发布时间', width: 180, render: (_, row) => formatDate(row.display_published_at) },
    { title: '学校', dataIndex: 'school_name', width: 150, ellipsis: true },
    { title: '昵称', dataIndex: 'author_nickname', width: 140, ellipsis: true },
    { title: '标签名称', width: 180, render: (_, r) => r.tags.join('、') || '—' },
    {
      title: '审核状态',
      width: 110,
      render: (_, r) => (
        <Tag color={{ pending: 'warning', approved: 'success', rejected: 'error' }[r.audit_status]}>
          {postAuditLabels[r.audit_status]}
        </Tag>
      ),
    },
    {
      title: '状态',
      width: 140,
      render: (_, row) => (
        <Dropdown
          trigger={['click']}
          disabled={status.disabled}
          menu={{
            selectedKeys: [row.visibility],
            items: (['visible', 'hidden', 'offline'] as const).map((visibility) => ({
              key: visibility,
              label: postStatusLabels[visibility],
              disabled: status.disabled || row.visibility === visibility,
              onClick: () => status.change(row, visibility),
            })),
          }}
        >
          <Button
            type="text"
            size="small"
            disabled={status.disabled}
            aria-label={`设置状态 ${row.display_label}`}
          >
            {postStatusLabels[row.visibility]} <DownOutlined />
          </Button>
        </Dropdown>
      ),
    },
    {
      title: '主图',
      width: 160,
      render: (_, r) =>
        r.cover_image ? (
          <ReviewImage school={school} source={r.cover_image} index={0} attachmentLabel="主图" />
        ) : (
          '—'
        ),
    },
    {
      title: '图集',
      width: 190,
      render: (_, r) => <PostGallery school={school} images={r.images} />,
    },
    { title: '创建时间', width: 180, render: (_, r) => formatDate(r.created_at) },
  ]
}

function PostGallery({ school, images }: { school: string; images: string[] }) {
  const [open, setOpen] = useState(false)
  if (!images.length) return '—'
  return (
    <Image.PreviewGroup>
      <details onToggle={(event) => setOpen(event.currentTarget.open)}>
        <summary>{images.length} 张图片</summary>
        {open &&
          images.map((source, index) => (
            <ReviewImage key={`${index}:${source}`} school={school} source={source} index={index} />
          ))}
      </details>
    </Image.PreviewGroup>
  )
}
