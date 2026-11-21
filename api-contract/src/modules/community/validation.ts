import { decodeResource, invalidDocument, type ResourceObject } from '../../http/protocol'
import type { CommunityPost, PostInteractionState } from './types'
function count(value: unknown) {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0
}
export function readPost(resource: ResourceObject, school: string, id?: string): CommunityPost {
  const value = decodeResource(resource)
  if (
    resource.type !== 'posts' ||
    value.school_id !== school ||
    (id !== undefined && value.id !== id) ||
    !count(value.like_count) ||
    !count(value.comment_count)
  )
    return invalidDocument()
  return value as unknown as CommunityPost
}
export function readInteractionState(
  resource: ResourceObject | ResourceObject[] | null,
  school: string,
  id: string,
): PostInteractionState {
  if (!resource || Array.isArray(resource) || resource.type !== 'post-interaction-states')
    return invalidDocument()
  const value = decodeResource(resource)
  if (
    value.id !== id ||
    value.school_id !== school ||
    !count(value.like_count) ||
    !count(value.comment_count) ||
    !['pending', 'approved', 'rejected'].includes(String(value.audit_status)) ||
    !['visible', 'hidden', 'offline', 'deleted'].includes(String(value.visibility)) ||
    ['liked', 'collected', 'author_followed', 'can_interact', 'can_share'].some(
      (key) => typeof value[key] !== 'boolean',
    )
  )
    return invalidDocument()
  return value as unknown as PostInteractionState
}
