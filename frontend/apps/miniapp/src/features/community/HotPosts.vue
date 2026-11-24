<script setup lang="ts">
import { onUnmounted, watch } from 'vue'
import { communityApi } from './api'
import { createHotPosts } from './use-hot-posts'
import { onPostInvalidated } from './post-invalidations'
import { schoolContext, scopeKey } from '../school-context/context'
import { session } from '../identity/session'
import { handlePageError, navigate } from '../../shared/use-page'

const props = defineProps<{ active: boolean }>()
const board = createHotPosts({
  scope: () => session.user && schoolContext.id
    ? { key: `${scopeKey()}:${session.user.id}`, school: schoolContext.id }
    : undefined,
  read: communityApi.hotPosts,
  failure: handlePageError,
  navigate: (school, post) => navigate(`/pages/index/detail?school=${school}&id=${post}`),
  reducedMotion: typeof globalThis.matchMedia === 'function' && globalThis.matchMedia('(prefers-reduced-motion: reduce)').matches,
})
const { items, pages, page, rowsPerPage, userPaused, touching, autoplay } = board
const stopInvalidations = onPostInvalidated(board.invalidate)
watch(() => [props.active, scopeKey(), session.user?.id], (_next, previous) => {
  if (props.active) void board.show(!previous?.[0])
  else board.hide()
}, { immediate: true, flush: 'sync' })
onUnmounted(() => { stopInvalidations(); board.dispose() })
defineExpose({
  show: () => props.active ? board.show(true) : Promise.resolve(),
  hide: board.hide,
  refresh: () => props.active ? (board.visible.value ? board.refresh() : board.show(true)) : Promise.resolve(),
})
</script>
<template>
  <view v-if="active && items.length" class="hot-posts">
    <view class="hot-posts-header">
      <text class="hot-posts-label">本校热榜</text>
      <button v-if="items.length > 1" size="mini" :aria-expanded="rowsPerPage === 3" @tap="board.setRowsPerPage(rowsPerPage === 1 ? 3 : 1)">{{ rowsPerPage === 1 ? '展开' : '收起' }}</button>
      <button v-if="pages.length > 1" size="mini" @tap="userPaused = !userPaused">{{ userPaused ? '继续播放' : '暂停播放' }}</button>
    </view>
    <swiper
      class="hot-posts-swiper"
      :style="{ height: `${Math.min(items.length, rowsPerPage) * 44}px` }"
      vertical
      :circular="pages.length > 1"
      :autoplay="autoplay"
      :interval="4000"
      :current="page"
      @change="board.changePage($event.detail.current)"
      @touchstart="touching = true"
      @touchend="touching = false"
      @touchcancel="touching = false"
    >
      <swiper-item v-for="(group, index) in pages" :key="index">
        <view v-for="item in group" :key="item.id" class="hot-post-row" role="button" :aria-label="`第${item.rank}名 ${item.display_label}`" @tap="board.open(item)">
          <text class="hot-post-rank">{{ item.rank }}</text>
          <view class="hot-post-body">
            <text class="hot-post-title">{{ item.display_label }}</text>
            <text class="hot-post-counts">赞 {{ item.like_count }} · 评论 {{ item.comment_count }}</text>
          </view>
        </view>
      </swiper-item>
    </swiper>
  </view>
</template>
<style scoped>
.hot-posts { padding: 0 16rpx; margin: 8rpx 0; background: #fff; border-radius: var(--linjian-radius-card); }
.hot-posts-header { display: flex; align-items: center; gap: 8rpx; }
.hot-posts-label { flex: 1; min-width: 0; font-size: var(--linjian-font-body); font-weight: 600; }
.hot-posts-header button { flex-shrink: 0; min-height: 44px; min-width: 44px; margin: 0; padding: 0 12rpx; line-height: 1.4; display: flex; align-items: center; justify-content: center; background: transparent; }
.hot-posts-header button::after { border: 0; }
.hot-post-row { display: flex; align-items: center; gap: 12rpx; height: 44px; }
.hot-post-rank { width: 36rpx; font-weight: 700; text-align: center; }
.hot-post-body { flex: 1; min-width: 0; display: flex; align-items: center; gap: 12rpx; }
.hot-post-counts { max-width: 45%; font-size: var(--linjian-font-caption); color: #6b7280; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.hot-post-title { display: block; flex: 1; min-width: 0; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>
