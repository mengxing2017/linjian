<script setup lang="ts">
import PageWindow from '../../shared/PageWindow.vue'
import { computed, onUnmounted, ref } from 'vue'
import { communityApi } from './api'
import { schoolContext, requireSchool, scopeKey, onScopeChange } from '../school-context/context'
import { session } from '../identity/session'
import { defaultTheme, themeColor } from '../school-context/branding'
import { handlePageError } from '../../shared/use-page'
import { notificationSummary } from './notification-state'
import { createNotificationList, notificationTabs, formatNotificationTime, notificationForeground, type NotificationTab } from './notification-list'

async function refreshSummary(force = false) {
  await notificationSummary.refresh(force)
  if (notificationSummary.state.error) throw new Error(notificationSummary.state.error)
}
const board = createNotificationList({
  scope: () => session.user && schoolContext.id ? { key: `${scopeKey()}:${session.user.id}`, school: schoolContext.id } : undefined,
  read: communityApi.notifications,
  markRead: communityApi.read,
  refreshSummary,
  invalidateSummary: async () => {
    await notificationSummary.invalidate()
    if (notificationSummary.state.error) throw new Error(notificationSummary.state.error)
  },
  failure: handlePageError,
})
const { state, current } = board
const stop = onScopeChange(board.reset)
onUnmounted(() => { stop(); board.dispose() })
const activeLabel = computed(() => notificationTabs.find(tab => tab.id === state.activeTab)!)
const selectedStyle = computed(() => {
  const school = schoolContext.schools.find(item => item.id === schoolContext.id)
  const primary = themeColor(school?.theme_primary, defaultTheme.theme_primary)
  const foreground = notificationForeground(primary, themeColor(school?.theme_text_color, defaultTheme.theme_text_color))
  return `--notification-tab-text:${foreground}`
})
const scrollTarget = computed(() => `notification-tab-${state.activeTab}`)
function select(tab: NotificationTab) {
  if (tab === state.activeTab) return
  void board.select(tab)
  uni.pageScrollTo({ scrollTop: 0, duration: 0 })
}
const labels: Record<string, string> = {
  like: '收到点赞', comment: '收到评论', follow: '有人关注了你', moderation: '审核结果',
}
let preparation = 0, disposed = false
const preparing = ref(false)
async function show(force = false) {
  const turn = ++preparation
  preparing.value = true
  try {
    const ready = await requireSchool()
    if (disposed || turn !== preparation) return
    preparing.value = false
    if (ready) await board.show(force)
  } catch (error) {
    if (!disposed && turn === preparation) state.categories[state.activeTab].error = handlePageError(error)
  } finally {
    if (turn === preparation) preparing.value = false
  }
}
onUnmounted(() => { disposed = true; preparation++ })
defineExpose({
  show,
  hide: () => { preparation++; preparing.value = false; board.hide() },
  refresh: () => show(true),
  loadMore: () => board.loadMore(),
})
</script>
<template>
  <view class="page compact-page notifications-page" :style="selectedStyle">
    <scroll-view class="notification-tabs" scroll-x :scroll-into-view="scrollTarget">
      <view class="notification-tab-track">
        <button v-for="tab in notificationTabs" :id="`notification-tab-${tab.id}`" :key="tab.id"
          class="notification-tab" :class="{ selected: state.activeTab === tab.id }"
          :aria-label="`${tab.label}${state.activeTab === tab.id ? '，已选中' : ''}`" @tap="select(tab.id)">
          {{ tab.label }}
        </button>
      </view>
    </scroll-view>
    <view v-if="state.readError" class="notification-error">标记已读失败：{{ state.readError }}，请重试对应消息。</view>
    <view v-if="state.summaryError" class="notification-hint">未读数量暂未更新，请下拉刷新重试。</view>
    <view v-if="current.loading || preparing" class="notification-hint">正在加载{{ activeLabel.label }}消息…</view>
    <PageWindow :page="current.page" :busy="current.loading || current.loadingMore" @previous="board.loadPage" />
    <view v-for="item in current.items" :key="item.id" class="notification-row" :class="{ unread: !item.read_at }">
      <view class="notification-copy">
        <view class="notification-title-line">
          <view v-if="!item.read_at" class="notification-unread-dot" />
          <text class="notification-title">{{ labels[item.kind] || '校园消息' }}</text>
          <text class="notification-read-state">{{ item.read_at ? '已读' : '未读' }}</text>
        </view>
        <text class="notification-time">{{ formatNotificationTime(item.created_at) }}</text>
      </view>
      <button class="notification-read" size="mini" :disabled="state.readingIds.includes(item.id) || !!item.read_at" @tap="board.read(item)">
        {{ item.read_at ? '已读' : state.readingIds.includes(item.id) ? '处理中…' : '标记已读' }}
      </button>
    </view>
    <view v-if="current.error" class="notification-error">
      <text>{{ current.error }}</text>
      <button size="mini" @tap="show(true)">重试</button>
    </view>
    <view v-else-if="!current.loading && !preparing && current.loaded && !current.items.length" class="notification-empty">
      <text>{{ activeLabel.empty }}</text>
      <text v-if="state.activeTab === 'all'" class="notification-hint">点赞、评论、关注和审核结果会显示在这里。</text>
    </view>
    <view v-if="current.appendError" class="notification-error">
      <text>加载失败：{{ current.appendError }}</text>
      <button size="mini" @tap="board.loadMore()">重试</button>
    </view>
    <button v-else-if="current.hasMore && !current.stale" :disabled="current.loadingMore" @tap="board.loadMore()">
      {{ current.loadingMore ? '正在加载…' : current.page % 3 === 0 ? '下一组消息' : '更多消息' }}
    </button>
    <view v-else-if="current.loaded && current.items.length && !current.loading && !current.error" class="notification-hint">没有更多消息了</view>
  </view>
</template>
<style scoped>
.notification-tabs { width: 100%; margin-bottom: 20rpx; border-radius: 28rpx; background: #fff; box-shadow: 0 6rpx 20rpx rgba(31, 41, 55, .05); }
.notification-tab-track { display: flex; min-width: 100%; width: max-content; padding: 8rpx; box-sizing: border-box; }
.notification-tab { flex: 1 0 auto; min-width: 44px; min-height: 44px; display: flex; align-items: center; justify-content: center; margin: 0; padding: 8rpx 18rpx; border-radius: 999rpx; background: transparent; color: #4b5563; font-size: var(--linjian-font-body); line-height: 1.5; white-space: nowrap; }
.notification-tab::after { border: 0; }
.notification-tab.selected { background: var(--primary); color: var(--notification-tab-text); font-weight: 600; }
.notification-row { display: flex; align-items: center; flex-wrap: wrap; gap: 8rpx 16rpx; padding: 16rpx 20rpx; margin-bottom: var(--linjian-space-list); border-radius: var(--linjian-radius-card); background: #fff; }
.notification-copy { flex: 1; min-width: 180rpx; overflow-wrap: anywhere; }
.notification-title-line { display: flex; align-items: center; flex-wrap: wrap; gap: 8rpx; }
.notification-title { font-size: var(--linjian-font-body); line-height: 1.5; }
.notification-time, .notification-read-state { font-size: var(--linjian-font-caption); line-height: 1.5; color: #6b7280; }
.notification-time { display: block; margin-top: 4rpx; }
.notification-unread-dot { width: 12rpx; height: 12rpx; border-radius: 50%; background: var(--primary); flex-shrink: 0; }
.notification-read { min-height: 44px; min-width: 44px; flex-shrink: 0; display: flex; align-items: center; justify-content: center; line-height: 1.4; margin: 0; }
.unread .notification-title { font-weight: 600; }
.notification-error { padding: 20rpx; color: #b91c1c; font-size: var(--linjian-font-caption); }
.notification-hint, .notification-empty { display: block; padding: 24rpx 0; text-align: center; color: #6b7280; font-size: var(--linjian-font-caption); }
</style>
