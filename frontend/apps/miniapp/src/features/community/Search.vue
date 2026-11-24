<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from 'vue'
import { communityApi, type CommunityPost } from './api'
import { createPostSearch } from './use-post-search'
import { requireSchool, schoolContext, scopeKey, onScopeChange } from '../school-context/context'
import { session } from '../identity/session'
import { handlePageError, navigate } from '../../shared/use-page'
import PostFeedCard from './PostFeedCard.vue'
const focus = ref(false)
const search = createPostSearch({
  api: communityApi,
  ensure: () => requireSchool(),
  scope: () =>
    schoolContext.id && session.user
      ? {
          key: `${scopeKey()}:${session.user.id}`,
          school: schoolContext.id,
          user: session.user.id,
        }
      : undefined,
  failure: handlePageError,
})
const {
  draftKeyword,
  submittedKeyword,
  posts,
  page,
  hasMore,
  loading,
  loadingMore,
  error,
  moreError,
  inputError,
  ready,
} = search
const school = computed(() => schoolContext.schools.find((item) => item.id === schoolContext.id))
const stop = onScopeChange(() => search.scopeChanged())
watch(
  () => session.user?.id,
  () => {
    search.clear()
    search.scopeChanged()
  },
  { flush: 'sync' },
)
let focused = false
function detail(post: CommunityPost) {
  navigate(
    `/pages/index/detail?school=${encodeURIComponent(schoolContext.id)}&id=${encodeURIComponent(post.id)}`,
  )
}
onUnmounted(() => {
  stop()
  search.dispose()
})
defineExpose({
  show: async () => {
    await search.show()
    if (search.visible.value && ready.value && !focused) {
      focused = true
      focus.value = true
    }
  },
  hide: () => {
    focus.value = false
    search.hide()
  },
  refresh: search.refresh,
  loadMore: search.loadMore,
})
</script>
<template>
  <view class="page compact-page post-search">
    <text class="muted">当前学校：{{ school?.name || '学校暂不可用' }}</text>
    <view class="row search-bar">
      <input
        v-model="draftKeyword"
        class="input grow"
        :disabled="!ready"
        :focus="focus"
        :maxlength="-1"
        placeholder="搜索帖子标题或正文"
        confirm-type="search"
        @confirm="search.submit()"
        @blur="focus = false"
      />
      <button v-if="draftKeyword || submittedKeyword" size="mini" @tap="search.clear()">
        清空
      </button>
      <button size="mini" :disabled="!ready" @tap="search.submit()">搜索</button>
    </view>
    <text v-if="inputError" class="muted" role="alert">{{ inputError }}</text>
    <text v-if="loading" class="search-status">搜索中…</text>
    <view v-else-if="error" class="search-status">
      <text>{{ error }}</text>
      <button size="mini" @tap="submittedKeyword ? search.refresh() : search.show()">重试</button>
    </view>
    <text v-else-if="!submittedKeyword" class="search-status">输入关键词开始搜索</text>
    <text v-else-if="page && !posts.length" class="search-status">
      暂无相关帖子，换个关键词试试
    </text>
    <view class="search-results">
      <PostFeedCard v-for="post in posts" :key="post.id" :post="post" :school="schoolContext.id" @open="detail" />
    </view>
    <text v-if="loadingMore" class="search-status">加载中…</text>
    <view v-else-if="moreError" class="search-status">
      <text>{{ moreError }}</text>
      <button size="mini" @tap="search.loadMore()">重试加载</button>
    </view>
    <button v-else-if="hasMore" :disabled="loading" @tap="search.loadMore()">加载更多</button>
    <text v-else-if="page && posts.length && !loading && !error" class="search-status">
      没有更多帖子了
    </text>
  </view>
</template>
<style scoped>
.search-bar {
  flex-wrap: nowrap;
}
.search-bar .input { width: auto; min-width: 0; }
.search-bar button { flex-shrink: 0; min-height: 44px; }
.search-status {
  display: block;
  text-align: center;
  padding: 32rpx 0;
}
.search-results { display: flex; flex-direction: column; gap: var(--linjian-space-list); }
</style>
