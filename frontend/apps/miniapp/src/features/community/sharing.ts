import type { PostInteractionState } from './api'

const imageUrl = '/static/images/share-cover.png'
export function postShare(state: PostInteractionState | undefined, school: string, id: string, enabled: boolean) {
  if (!enabled || !state || !state.can_share || state.id !== id || state.school_id !== school || state.audit_status !== 'approved' || state.visibility !== 'visible')
    return { title: 'Linjian', path: '/pages/index/index', imageUrl }
  return { title: '在 Linjian 查看帖子', path: `/pages/index/detail?school=${school}&id=${id}`, imageUrl }
}
