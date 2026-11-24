<script setup lang="ts">
import { shallowRef, ref } from 'vue'
import { communityApi, type Notification } from './api'
import { schoolContext, requireSchool, scopeKey } from '../school-context/context'
import { usePage } from '../../shared/use-page'
import PageStatus from '../../shared/PageStatus.vue'
import { notificationSummary } from './notification-state'
const rows = shallowRef<Notification[]>([]),
  page = ref(1),
  more = ref(false)
const labels: Record<string, string> = {
  like: '收到点赞',
  comment: '收到评论',
  follow: '有人关注了你',
  moderation: '审核结果',
}
let loaded = false
let loadedUnread: number | undefined
const { busy, error, run } = usePage(() => {
  loaded = false
  loadedUnread = undefined
  rows.value = []
  page.value = 1
  more.value = false
})
function load(reset = true) {
  return run(async () => {
    if (!(await requireSchool())) return
    const key = scopeKey(),
      p = reset ? 1 : page.value + 1
    const result = await communityApi.notifications(schoolContext.id, p)
    if (key !== scopeKey()) return
    rows.value = reset ? result.items : [...new Map([...rows.value, ...result.items].map(row => [row.id, row])).values()]
    page.value = p
    more.value = !!result.page?.has_more
    loaded = true
    loadedUnread = notificationSummary.state.count
  })
}
function read(item: Notification) {
  if (item.read_at) return
  void run(async (current) => {
    await communityApi.read(schoolContext.id, item.id)
    if (current()) {
      rows.value = rows.value.map((row) =>
        row.id === item.id ? { ...row, read_at: new Date().toISOString() } : row,
      )
      await notificationSummary.invalidate()
      if (current()) loadedUnread = notificationSummary.state.count
    }
  })
}
defineExpose({
  show: async () => {
    const key = scopeKey()
    if (!loaded) {
      await Promise.all([load(), notificationSummary.refresh()])
      if (key === scopeKey() && loaded) loadedUnread = notificationSummary.state.count
      return
    }
    await notificationSummary.refresh()
    if (key === scopeKey() && notificationSummary.state.count !== undefined && notificationSummary.state.count !== loadedUnread) await load()
  },
  refresh: async () => {
    const key = scopeKey()
    await Promise.all([load(), notificationSummary.refresh(true)])
    if (key === scopeKey() && loaded) loadedUnread = notificationSummary.state.count
  },
  loadMore: () => {
    if (more.value) return load(false)
  },
})
</script>
<template>
  <view class="page">
    <text class="heading">消息</text>
    <view v-for="item in rows" :key="item.id" class="card">
      <text class="title">{{ labels[item.kind] || '校园消息' }}</text>
      <text class="muted">{{ item.created_at }}</text>
      <button size="mini" :disabled="busy || !!item.read_at" @tap="read(item)">
        {{ item.read_at ? '已读' : '标记已读' }}
      </button>
    </view>
    <PageStatus :busy="busy" :error="error" :empty="!rows.length" @retry="load()" />
    <button v-if="more" :disabled="busy" @tap="load(false)">更多消息</button>
  </view>
</template>
