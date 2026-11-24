import { watch } from 'vue'
import { session } from '../features/identity/session'
import { scopeKey, schoolContext } from '../features/school-context/context'
import { schoolTheme } from '../features/school-context/theme'
import { notificationSummary } from '../features/community/notification-state'
import { notificationBadge } from '../features/community/notification-summary'
import { currentRoot, isRootId, rootRoutes } from './root-routes'
import { openPublish, switchRoot } from './navigation'
import { iconPath } from '../shared/icons'

interface NativeTabbar { setData: (data: Record<string, unknown>) => void }
interface TabbarBridge {
  attach: (bar: NativeTabbar) => void
  detach: (bar: NativeTabbar) => void
  activate: (id: unknown) => void
}
const bars = new Map<NativeTabbar, Record<string, unknown>>()
const items = [...rootRoutes.slice(0, 2), { id: 'publish', label: '发布动态' }, ...rootRoutes.slice(2)]
  .map(item => ({ ...item, icon: iconPath(item.id), selectedIcon: iconPath(item.id, true) }))
function update() {
  const data = {
    items,
    selected: currentRoot() ?? '',
    ready: !!session.user?.bound_school_id,
    badge: notificationBadge(notificationSummary.state.count),
    theme: schoolTheme.value,
  }
  bars.forEach((previous, bar) => {
    const changed = Object.fromEntries(Object.entries(data).filter(([key, value]) => previous[key] !== value))
    if (!Object.keys(changed).length) return
    bar.setData(changed)
    bars.set(bar, data)
  })
}
export function installTabbar(app: { linjianTabbar?: TabbarBridge }) {
  app.linjianTabbar = {
    attach(bar) { if (!bars.has(bar)) bars.set(bar, {}); update(); void notificationSummary.refresh() },
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
