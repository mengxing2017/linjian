import { decodeResource, invalidDocument, type ResourceDocument } from '../../http/protocol'

export interface NotificationSummary {
  id: string
  school_id: string
  unread_count: number
}

export function readNotificationSummary(document: ResourceDocument, school: string): NotificationSummary {
  const resource = document.data
  if (!resource || Array.isArray(resource) || resource.type !== 'notification-summaries' || resource.id !== school) return invalidDocument()
  const value = decodeResource(resource)
  if (value.school_id !== school || typeof value.unread_count !== 'number' || !Number.isSafeInteger(value.unread_count) || value.unread_count < 0) return invalidDocument()
  return value as unknown as NotificationSummary
}
