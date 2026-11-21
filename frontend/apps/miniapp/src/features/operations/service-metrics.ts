import { serviceApi, type OperationEvent, type OperationOutcome } from './api'
import { scopeKey } from '../school-context/context'
async function deliver(send: () => Promise<unknown>, scope: string) {
  for (const delay of [0, 300, 1500]) {
    if (delay) await new Promise(resolve => globalThis.setTimeout(resolve, delay))
    if (scope !== scopeKey()) return false
    try { await send(); return true } catch (error) {
      const status = error && typeof error === 'object' && 'status' in error ? Number(error.status) : 0
      if (status >= 400 && status < 500 && status !== 409) return false
    }
  }
  return false
}
export function sendServiceEvent(school: string, event: OperationEvent) {
  const scope = scopeKey()
  return deliver(() => serviceApi.event(school, event), scope)
}
export function sendServiceOutcome(school: string, scope: string, click: Promise<boolean>, event: OperationOutcome) {
  void click.then(accepted => accepted && scope === scopeKey() ? deliver(() => serviceApi.outcome(school, event), scope) : false)
}
