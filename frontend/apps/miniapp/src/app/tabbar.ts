import { watch } from 'vue'
import { session } from '../features/identity/session'
import { scopeKey, schoolContext } from '../features/school-context/context'
import { schoolTheme } from '../features/school-context/theme'
import { notificationSummary } from '../features/community/notification-state'
import { notificationBadge } from '../features/community/notification-summary'
import { currentRoot, isRootId, rootRoutes } from './root-routes'
import { openPublish, switchRoot } from './navigation'

interface NativeTabbar { setData: (data: Record<string, unknown>) => void }
interface TabbarBridge {
  attach: (bar: NativeTabbar) => void
  detach: (bar: NativeTabbar) => void
  activate: (id: unknown) => void
}
const bars = new Set<NativeTabbar>()
const items = [...rootRoutes.slice(0, 2), { id: 'publish', label: '发布动态' }, ...rootRoutes.slice(2)]
function update() {
  const data = {
    items,
    selected: currentRoot() ?? '',
    ready: !!session.user?.bound_school_id,
    badge: notificationBadge(notificationSummary.state.count),
    theme: schoolTheme.value,
  }
  bars.forEach(bar => bar.setData(data))
}
export function installTabbar(app: { linjianTabbar?: TabbarBridge }) {
  app.linjianTabbar = {
    attach(bar) { bars.add(bar); update(); void notificationSummary.refresh() },
    detach(bar) { bars.delete(bar) },
    activate(id) {
      const operation = id === 'publish' ? openPublish() : isRootId(id) ? switchRoot(id) : undefined
      void operation?.then(update).catch(() => uni.showToast({ title: '页面未能打开，请重试', icon: 'none' }))
    },
  }
  watch(() => [scopeKey(), session.user?.id], () => {
    notificationSummary.sync()
    update()
  }, { flush: 'sync' })
  watch(() => [session.user?.id, schoolContext.id], () => { void notificationSummary.refresh() })
  watch(() => [notificationSummary.state.count, schoolTheme.value, session.user?.bound_school_id], update, { flush: 'sync' })
}
