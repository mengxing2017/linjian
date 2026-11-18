import { session } from '../identity/session'
import { statisticsApi, type ActivityInput } from './api'
let sequence = 0
export function recordPage(kind: ActivityInput['kind'], school: string, post?: string) {
  if (!session.user || !school) return
  const user = session.user.id
  // UUID v4 shape; this identifier is only for deduplication, never authorization.
  const event_id = 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (char) => {
    const random = (Math.random() * 16 + sequence++) % 16 | 0
    return (char === 'x' ? random : (random & 3) | 8).toString(16)
  })
  const input: ActivityInput = {
    event_id,
    kind,
    school_id: school,
    ...(post ? { post_id: post } : {}),
  }
  void statisticsApi.activity(input).catch(() => {
    setTimeout(() => {
      if (session.user?.id === user) void statisticsApi.activity(input).catch(() => undefined)
    }, 1000)
  })
}
