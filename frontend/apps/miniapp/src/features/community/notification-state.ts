import { communityApi } from './api'
import { createNotificationSummary } from './notification-summary'
import { session } from '../identity/session'
import { schoolContext, scopeKey } from '../school-context/context'
import { handlePageError } from '../../shared/use-page'

export const notificationSummary = createNotificationSummary({
  scope: () => session.user?.bound_school_id && schoolContext.id
    ? { key: `${scopeKey()}:${session.user.id}`, school: schoolContext.id }
    : undefined,
  read: communityApi.notificationSummary,
  failure: handlePageError,
})
