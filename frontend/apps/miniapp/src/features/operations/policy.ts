import type { OperationResource } from './api'
export function uuid() {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, c => { const r = Math.floor(Math.random() * 16); return (c === 'x' ? r : (r & 3) | 8).toString(16) })
}
export function shanghaiDay(time: number) { return Math.floor((time + 8 * 3600000) / 86400000) }
export function shouldPopup(v: Pick<OperationResource, 'popup_strategy' | 'popup_interval_days'>, last: number, now: number) {
  if (v.popup_strategy === 'always' || !last) return true
  if (v.popup_strategy === 'daily') return shanghaiDay(last) !== shanghaiDay(now)
  return now - last >= v.popup_interval_days * 86400000
}
export function popupKey(user: string, school: string, resource: string) { return `linjian:miniapp:group-qrcode:${user}:${school}:${resource}` }
