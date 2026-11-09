import { Space, Tabs } from 'antd'
import { useSearchParams } from 'react-router-dom'
import type { School } from './api'
import { AuditModes } from './audit-modes'
import { CommentReviews } from './comment-reviews'
import { PostManagement } from './post-management'

export function ContentModeration({ school, canEditModes, platformAdmin = false, publisher, publisherId, refresh }: { school: School; canEditModes: boolean; platformAdmin?: boolean; publisher?: string; publisherId?: string; refresh: () => Promise<void> }) {
  const [params, setParams] = useSearchParams()
  return <Space orientation="vertical" className="full-width">
    <AuditModes school={school} canEdit={canEditModes} />
    <Tabs destroyOnHidden activeKey={params.get('subview') === 'comments' ? 'comments' : 'posts'} onChange={key => setParams(previous => {
      const next = new URLSearchParams(previous)
      next.set('subview', key)
      return next
    })} items={[
      { key: 'posts', label: '帖子管理', children: <PostManagement publisherId={publisherId} key={school.id} context={`学校：${school.name}；发布人：${publisher || "当前登录账号"}；帖子审核模式：${school.config.post_audit_mode === "auto" ? "自动审核" : "人工审核"}`} school={school.id} platformAdmin={platformAdmin} /> },
      { key: 'comments', label: '评论审核', children: <CommentReviews school={school} refresh={refresh} /> },
    ]} />
  </Space>
}
