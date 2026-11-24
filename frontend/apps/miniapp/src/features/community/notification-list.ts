import { computed, shallowReactive } from 'vue'
import { mergePageWindow } from '../../shared/page-window'
import type { Notification, NotificationFilter, NotificationKind } from './api'

export type NotificationTab = 'all' | NotificationKind
export const notificationTabs: ReadonlyArray<{ id: NotificationTab; label: string; empty: string }> = [
  { id: 'all', label: '全部', empty: '暂时没有消息' },
  { id: 'comment', label: '评论', empty: '暂无评论通知' },
  { id: 'like', label: '点赞', empty: '暂无点赞通知' },
  { id: 'follow', label: '关注', empty: '暂无关注通知' },
  { id: 'moderation', label: '系统', empty: '暂无审核结果通知' },
]

interface ListScope { key: string; school: string }
function category() {
  return {
    items: [] as Notification[], page: 0, hasMore: false, loaded: false, stale: false,
    loading: false, loadingMore: false, error: '', appendError: '', requestVersion: 0,
  }
}
function categories() {
  return Object.fromEntries(notificationTabs.map(tab => [tab.id, shallowReactive(category())])) as Record<NotificationTab, ReturnType<typeof category>>
}

export function createNotificationList(options: {
  scope: () => ListScope | undefined
  read: (school: string, page: number, filter: NotificationFilter) => Promise<{ items: Notification[]; page?: { has_more: boolean } }>
  markRead: (school: string, id: string) => Promise<unknown>
  refreshSummary: (force: boolean) => Promise<void>
  invalidateSummary: () => Promise<void>
  failure: (error: unknown) => string
  now?: () => Date
}) {
  const state = shallowReactive({
    activeTab: 'all' as NotificationTab, categories: categories(),
    readingIds: [] as string[], readError: '', summaryError: '',
  })
  const current = computed(() => state.categories[state.activeTab])
  let scope: ListScope | undefined
  let epoch = 0, disposed = false, visible = true, summaryVersion = 0
  const acknowledgedReads = new Map<string, string>()

  function reset() {
    epoch++
    summaryVersion++
    scope = options.scope()
    state.activeTab = 'all'
    state.categories = categories()
    state.readingIds = []
    state.readError = ''
    state.summaryError = ''
    acknowledgedReads.clear()
  }
  function sync() {
    if (scope?.key !== options.scope()?.key) reset()
    return scope
  }
  function valid(turn: number, key: string) {
    sync()
    return !disposed && turn === epoch && key === scope?.key
  }
  function invalidate(tab: NotificationTab) {
    const list = state.categories[tab]
    list.requestVersion++
    list.loading = false
    list.loadingMore = false
  }
  async function load(resetPage = true, targetPage?: number) {
    if (disposed || !visible) return
    const active = sync()
    if (!active) return
    const tab = state.activeTab, list = state.categories[tab]
    if (!resetPage && (list.loading || list.loadingMore || !list.loaded || list.stale || (!targetPage && !list.hasMore))) return
    if (resetPage) invalidate(tab)
    const version = ++list.requestVersion, turn = epoch, page = targetPage ?? (resetPage ? 1 : list.page + 1)
    const accepts = () => valid(turn, active.key) && visible && state.activeTab === tab && list.requestVersion === version
    list.loading = resetPage
    list.loadingMore = !resetPage
    list.error = ''
    list.appendError = ''
    if (resetPage) list.stale = true
    try {
      const result = await options.read(active.school, page, tab === 'all' ? {} : { kind: tab })
      if (!accepts()) return
      const items = mergePageWindow(resetPage ? [] : list.items, resetPage ? 0 : list.page, result.items, page)
      list.items = [...new Map(items.map(item => {
        const readAt = acknowledgedReads.get(item.id)
        return [item.id, readAt && !item.read_at ? { ...item, read_at: readAt } : item]
      })).values()]
      list.page = page
      list.hasMore = !!result.page?.has_more
      list.loaded = true
      list.stale = false
      const retained = new Set(notificationTabs.flatMap(entry => state.categories[entry.id].items.map(item => item.id)))
      for (const id of acknowledgedReads.keys()) if (!retained.has(id)) acknowledgedReads.delete(id)
    } catch (error) {
      if (!accepts()) return
      const message = options.failure(error)
      if (!accepts()) return
      if (resetPage) list.error = message
      else list.appendError = message
    } finally {
      if (accepts()) {
        list.loading = false
        list.loadingMore = false
      }
    }
  }
  async function summary(force: boolean) {
    const active = sync()
    if (!active || disposed) return
    const turn = epoch, version = ++summaryVersion
    state.summaryError = ''
    try {
      await options.refreshSummary(force)
    } catch (error) {
      if (valid(turn, active.key) && version === summaryVersion) {
        const message = options.failure(error)
        if (valid(turn, active.key) && version === summaryVersion) state.summaryError = message
      }
    }
  }
  async function select(tab: NotificationTab) {
    sync()
    if (disposed || tab === state.activeTab) return
    invalidate(state.activeTab)
    state.activeTab = tab
    if (!current.value.loaded || current.value.stale) await load()
  }
  async function read(item: Notification) {
    const active = sync()
    if (!active || disposed || item.read_at || acknowledgedReads.has(item.id) || state.readingIds.includes(item.id)) return
    if (!current.value.items.some(row => row.id === item.id)) return
    const turn = epoch
    state.readingIds = [...state.readingIds, item.id]
    state.readError = ''
    try {
      await options.markRead(active.school, item.id)
      if (!valid(turn, active.key)) return
      const readAt = (options.now?.() ?? new Date()).toISOString()
      acknowledgedReads.set(item.id, readAt)
      for (const tab of notificationTabs) {
        const list = state.categories[tab.id]
        list.items = list.items.map(row => row.id === item.id && !row.read_at ? { ...row, read_at: readAt } : row)
      }
    } catch (error) {
      if (valid(turn, active.key)) {
        const message = options.failure(error)
        if (valid(turn, active.key)) state.readError = message
      }
      return
    } finally {
      if (valid(turn, active.key)) state.readingIds = state.readingIds.filter(id => id !== item.id)
    }
    const version = ++summaryVersion
    state.summaryError = ''
    try {
      await options.invalidateSummary()
    } catch (error) {
      if (valid(turn, active.key) && version === summaryVersion) {
        const message = options.failure(error)
        if (valid(turn, active.key) && version === summaryVersion) state.summaryError = message
      }
    }
  }
  function hide() {
    visible = false
    for (const tab of notificationTabs) {
      invalidate(tab.id)
      state.categories[tab.id].stale = true
    }
  }
  async function show(force = false) {
    if (disposed) return
    sync()
    visible = true
    if (force) for (const tab of notificationTabs) state.categories[tab.id].stale = true
    await Promise.all([load(), summary(force)])
  }
  async function refresh() {
    if (disposed) return
    sync()
    for (const tab of notificationTabs) state.categories[tab.id].stale = true
    await Promise.all([load(), summary(true)])
  }
  function dispose() {
    hide()
    reset()
    disposed = true
  }
  return { state, current, select, read, show, hide, refresh, reset, dispose, loadMore: () => load(false), loadPage: (page: number) => load(false, page), retry: () => load() }
}

export function formatNotificationTime(value: string, now = new Date()) {
  const date = new Date(value)
  if (!value || !Number.isFinite(date.getTime())) return '时间未知'
  const pad = (number: number) => String(number).padStart(2, '0')
  const time = `${pad(date.getHours())}:${pad(date.getMinutes())}`
  const sameYear = date.getFullYear() === now.getFullYear()
  if (sameYear && date.getMonth() === now.getMonth() && date.getDate() === now.getDate()) return time
  return `${sameYear ? '' : `${date.getFullYear()}年`}${date.getMonth() + 1}月${date.getDate()}日 ${time}`
}

export function notificationForeground(background: string, preferred: string) {
  const luminance = (color: string) => {
    const channels = [1, 3, 5].map(offset => parseInt(color.slice(offset, offset + 2), 16) / 255)
      .map(channel => channel <= 0.04045 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4)
    return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722
  }
  const base = luminance(background), text = luminance(preferred)
  if ((Math.max(base, text) + 0.05) / (Math.min(base, text) + 0.05) >= 4.5) return preferred
  return (base + 0.05) / 0.05 >= 1.05 / (base + 0.05) ? '#000000' : '#FFFFFF'
}
