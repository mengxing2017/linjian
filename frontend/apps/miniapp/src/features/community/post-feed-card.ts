import type { CommunityPost } from './api'
import { postVisibilityLabel } from './post-status'

export function postFeedCopy(post: Pick<CommunityPost, 'title' | 'content'>) {
  const title = post.title?.trim() ?? ''
  const content = post.content.trim()
  const normalizedTitle = title.replace(/\s+/g, '')
  const normalizedContent = content.replace(/\s+/g, '')
  return {
    title: content && normalizedTitle !== normalizedContent ? title : '',
    summary: content || title,
  }
}

export function postFeedStatus(post: Pick<CommunityPost, 'visibility' | 'audit_status'>) {
  const audit =
    post.audit_status === 'pending'
      ? '待审核'
      : post.audit_status === 'rejected'
        ? '未通过审核'
        : '已通过审核'
  return `状态：${postVisibilityLabel(post.visibility)} · ${audit}`
}
