import { reactive } from 'vue'
import type { NotificationSummary } from './api'

interface SummaryScope { key: string; school: string }
export function notificationBadge(count: number | undefined) {
  return count !== undefined && Number.isSafeInteger(count) && count > 0 ? (count > 99 ? '99+' : String(count)) : ''
}
export function createNotificationSummary(options: {
  scope: () => SummaryScope | undefined
  read: (school: string) => Promise<NotificationSummary>
  failure: (error: unknown) => void
  now?: () => number
}) {
  const state = reactive({ count: undefined as number | undefined, busy: false })
  const now = options.now ?? Date.now
  let scope: SummaryScope | undefined
  let generation = 0, mutation = 0, validatedAt: number | undefined, pending = false
  let flight: Promise<void> | undefined
  function sync() {
    const next = options.scope()
    if (next?.key !== scope?.key) {
      generation++
      scope = next
      state.count = undefined
      state.busy = false
      validatedAt = undefined
      pending = false
      flight = undefined
    }
    return scope
  }
  function refresh(force = false): Promise<void> {
    const active = sync()
    if (!active) return Promise.resolve()
    if (force) pending = true
    if (flight) return flight
    if (!force && validatedAt !== undefined && now() - validatedAt < 30_000) return Promise.resolve()
    pending = true
    const turn = generation
    state.busy = true
    const operation = (async () => {
      while (pending && turn === generation) {
        pending = false
        const version = mutation
        try {
          const result = await options.read(active.school)
          sync()
          if (turn === generation && version === mutation) {
            state.count = result.unread_count
            validatedAt = now()
          }
        } catch (error) {
          sync()
          if (turn === generation && version === mutation) {
            validatedAt = undefined
            options.failure(error)
          }
        }
      }
    })()
    flight = operation
    void operation.finally(() => {
      if (flight === operation) { flight = undefined; state.busy = false }
    })
    return operation
  }
  function invalidate() {
    sync()
    mutation++
    validatedAt = undefined
    return refresh(true)
  }
  return { state, sync, refresh, invalidate }
}
