export type MarketingStage = 'loaded' | 'error' | 'show_api_success' | 'closed_early' | 'client_completed'
export interface RewardedVideo {
  show: () => Promise<unknown>; load: () => Promise<unknown>; destroy: () => void
  onLoad: (listener: () => void) => void; offLoad: (listener: () => void) => void
  onError: (listener: () => void) => void; offError: (listener: () => void) => void
  onClose: (listener: (result: { isEnded?: boolean }) => void) => void
  offClose: (listener: (result: { isEnded?: boolean }) => void) => void
}
export function rewardLifecycle(ad: RewardedVideo, callbacks: { current: () => boolean; record: (stage: MarketingStage) => void; completed: () => void; failed: () => void; closed: () => void }) {
  let disposed = false, running = false, granted = false, errorRecorded = false
  const current = () => !disposed && callbacks.current()
  const load = () => { if (current()) callbacks.record('loaded') }
  const error = () => { if (!current() || errorRecorded) return; errorRecorded = true; running = false; callbacks.record('error'); callbacks.failed() }
  const close = (result: { isEnded?: boolean }) => {
    if (!current() || !running) return
    running = false; callbacks.closed()
    if (result?.isEnded === true) {
      callbacks.record('client_completed')
      if (!granted) { granted = true; callbacks.completed() }
    } else callbacks.record('closed_early')
  }
  ad.onLoad(load); ad.onError(error); ad.onClose(close)
  return {
    show() {
      if (!current() || running || granted) return Promise.resolve(false)
      running = true; errorRecorded = false
      try {
        return ad.show().then(() => { if (!current()) return false; callbacks.record('show_api_success'); return true }).catch(() => { error(); if (current()) void ad.load().catch(() => {}); return false })
      } catch { error(); return Promise.resolve(false) }
    },
    dispose() { if (disposed) return; disposed = true; running = false; ad.offLoad(load); ad.offError(error); ad.offClose(close); ad.destroy() },
  }
}
