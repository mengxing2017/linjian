import type { CommunityPost } from './api'
import { postVisibilityLabel } from './post-status'

export const FEED_SUMMARY_LENGTH = 240

export function feedExcerpt(value: string, limit = FEED_SUMMARY_LENGTH) {
  let excerpt = ''
  let count = 0
  for (const character of value) {
    if (count === limit) return `${excerpt}…`
    excerpt += character
    count++
  }
  return excerpt
}

export function feedPost(post: CommunityPost): CommunityPost {
  return { ...post, content: feedExcerpt(post.content) }
}

export function postFeedCopy(post: Pick<CommunityPost, 'title' | 'content'>) {
  const title = post.title?.trim() ?? ''
  const content = post.content.trim()
  const normalizedTitle = title.replace(/\s+/g, '')
  const normalizedContent = content.replace(/\s+/g, '')
  return {
    title: content && normalizedTitle !== normalizedContent ? title : '',
    summary: feedExcerpt(content || title),
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
