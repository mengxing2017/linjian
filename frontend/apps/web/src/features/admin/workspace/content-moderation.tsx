import { Space, Tabs } from 'antd'
import { useSearchParams } from 'react-router-dom'
import type { School } from './api'
import { AuditModes } from './audit-modes'
import { CommentReviews } from './comment-reviews'
import { PostReviews } from './post-reviews'

export function ContentModeration({ school, canEditModes, refresh }: { school: School; canEditModes: boolean; refresh: () => Promise<void> }) {
  const [params, setParams] = useSearchParams()
  return <Space orientation="vertical" className="full-width">
    <AuditModes school={school} canEdit={canEditModes} />
    <Tabs activeKey={params.get('subview') === 'comments' ? 'comments' : 'posts'} onChange={key => setParams(previous => {
      const next = new URLSearchParams(previous)
      next.set('subview', key)
      return next
    })} items={[
      { key: 'posts', label: '帖子审核', children: <PostReviews school={school.id} /> },
      { key: 'comments', label: '评论审核', children: <CommentReviews school={school} refresh={refresh} /> },
    ]} />
  </Space>
}
