import { useState } from 'react'
import { Button, Image, Tag } from 'antd'
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
): TableColumnsType<PostManagementView> {
  return [
    {
      title: '标题',
      width: 220,
      render: (_, r) => (
        <Button type="link" className="record-title review-title" onClick={() => detail(r.id)}>
          {r.title}
        </Button>
      ),
    },
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
    { title: '置顶状态', width: 110, render: (_, r) => (r.is_top ? '已置顶' : '未置顶') },
    { title: '状态', width: 110, render: (_, r) => postStatusLabels[r.visibility] },
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
