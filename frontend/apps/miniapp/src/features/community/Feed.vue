<script setup lang="ts">
import { mergePageWindow } from '../../shared/page-window'
import PageWindow from '../../shared/PageWindow.vue'
import { feedPost } from './post-feed-card'
import { mapConcurrent } from '../../shared/concurrency'
import { recordPage } from '../statistics/activity'
import HomeOperations from '../operations/HomeOperations.vue'
import HomeShortcuts from '../operations/HomeShortcuts.vue'
import HotPosts from './HotPosts.vue'
import { isHotPostsHome } from './use-hot-posts'
const hotPosts = shallowRef<InstanceType<typeof HotPosts>>()
const pageVisible = ref(false)
let pageTurn = 0
const shortcuts = shallowRef<InstanceType<typeof HomeShortcuts>>()
const operations = shallowRef<InstanceType<typeof HomeOperations>>()
import { shallowRef, ref, computed, watch, onUnmounted } from 'vue'
import { communityApi, homeMenuApi, type HomeMenu, type CommunityPost, type FeedFilter, type PostInteractionState } from './api'
import { requireSchool, schoolContext, scopeKey } from '../school-context/context'
import { invalidatedPosts, acknowledgePost, clearPostInvalidations } from './post-invalidations'
import { session } from '../identity/session'
import { usePage, navigate } from '../../shared/use-page'
import PageStatus from '../../shared/PageStatus.vue'
import AppIcon from '../../shared/AppIcon.vue'
import PostFeedCard from './PostFeedCard.vue'
import SchoolHeader from '../school-context/SchoolHeader.vue'
const props = defineProps<{
  initial?: 'feed' | 'following' | 'mine' | 'collect'
  searchPage?: boolean
  homePage?: boolean
  lockedMode?: boolean
}>()
const mode = ref(props.initial ?? 'feed'),
  search = ref(''),
  posts = shallowRef<CommunityPost[]>([]),
  page = ref(1),
  more = ref(false),
  menu = shallowRef<HomeMenu>(),
  selectedTag = ref('')
watch(
  () => props.initial,
  (value) => {
    mode.value = value ?? 'feed'
  },
)
const school = computed(() => schoolContext.school)
const ownPostsPage = computed(() => props.lockedMode && props.initial === 'mine')
const hotPostsActive = computed(() => pageVisible.value && !!session.user && !!school.value && isHotPostsHome(!!props.homePage, !!props.searchPage, mode.value, search.value, selectedTag.value))
const { busy, error, run } = usePage(() => {
  clearPostInvalidations()
  posts.value = []
  page.value = 1
  more.value = false
  menu.value = undefined
  selectedTag.value = ''
})
function load(reset = true, retryAfterScopeRefresh = true, targetPage?: number, schoolReady = false): Promise<void> {
  return run(async (current) => {
    if (!schoolReady && !(await requireSchool())) return
    if (!current()) {
      if (retryAfterScopeRefresh) void load(true, false)
      return
    }
    const id = schoolContext.id,
      generation = schoolContext.generation,
      user = session.user?.id
    const scope = `${scopeKey()}:${user ?? ''}`
    const dirty = invalidatedPosts(scope, id)
    if (reset && !ownPostsPage.value) {
      const loaded = await homeMenuApi.public(id)
      if (!current() || generation !== schoolContext.generation || id !== schoolContext.id || user !== session.user?.id) return
      menu.value = loaded
      if (selectedTag.value && !loaded.tags.some(tag => tag.name === selectedTag.value)) selectedTag.value = ''
    }
    const next = targetPage ?? (reset ? 1 : page.value + 1)
    const kind: FeedFilter['kind'] =
      !ownPostsPage.value && (mode.value === 'collect' || mode.value === 'following') ? mode.value : undefined
    const response = await communityApi.posts(id, {
      page: next,
      search: search.value,
      mine: ownPostsPage.value || mode.value === 'mine',
      kind,
      tag: ownPostsPage.value ? '' : selectedTag.value,
    })
    if (
      current() &&
      generation === schoolContext.generation &&
      id === schoolContext.id &&
      user === session.user?.id
    ) {
      posts.value = mergePageWindow(reset ? [] : posts.value, reset ? 0 : page.value, response.items.map(feedPost), next)
      page.value = next
      more.value = !!response.page?.has_more
      if (reset) dirty.forEach(item => acknowledgePost(scope, id, item.id, item.version))
    }
  })
}
function setMode(value: typeof mode.value) {
  if (busy.value || props.lockedMode) return
  mode.value = value
  void load()
}
function selectTag(tag: string) {
  if (busy.value) return
  selectedTag.value = tag
  void load()
}
async function syncInvalidated(current: () => boolean) {
  const school = schoolContext.id
  const scope = `${scopeKey()}:${session.user?.id ?? ''}`
  const valid = () => current() && scope === `${scopeKey()}:${session.user?.id ?? ''}`
  const ids = new Set(posts.value.map(item => item.id))
  const dirty = invalidatedPosts(scope, school).filter(item => ids.has(item.id))
  if (!dirty.length) return
  const results = await mapConcurrent(dirty, async item => {
    try { return { item, state: await communityApi.interactionState(school, item.id), cause: undefined } }
    catch (cause) { return { item, state: undefined, cause } }
  }, valid)
  if (!valid()) return
  const latest = new Map(invalidatedPosts(scope, school).map(item => [item.id, item.version]))
  const changes = new Map<string, PostInteractionState | null>()
  const failures: unknown[] = []
  for (const { item, state, cause } of results) {
    if (latest.get(item.id) !== item.version) continue
    if (state) {
      const readable = mode.value === 'mine' || (state.audit_status === 'approved' && state.visibility === 'visible')
      const matches = readable && (mode.value !== 'collect' || state.collected) && (mode.value !== 'following' || state.author_followed)
      changes.set(item.id, matches ? state : null)
    } else if (cause && typeof cause === 'object' && 'status' in cause && cause.status === 404) {
      changes.set(item.id, null)
    } else { failures.push(cause); continue }
    acknowledgePost(scope, school, item.id, item.version)
  }
  if (changes.size) posts.value = posts.value.flatMap(item => {
    const change = changes.get(item.id)
    return change === null ? [] : [change ? { ...item, ...change } : item]
  })
  if (failures.length) throw failures[0]
}
function detail(post: CommunityPost) {
  navigate(`/pages/index/detail?school=${schoolContext.id}&id=${post.id}`)
}
onUnmounted(() => { pageTurn++; pageVisible.value = false; hotPosts.value?.hide() })
defineExpose({
  hide: () => { pageTurn++; pageVisible.value = false; hotPosts.value?.hide(); operations.value?.hide(); shortcuts.value?.hide() },
  show: async () => {
    const turn = ++pageTurn
    if (!(await requireSchool())) return
    if (turn !== pageTurn) return
    pageVisible.value = true
    void hotPosts.value?.show()
    void operations.value?.show()
    void shortcuts.value?.show()
    const activeSchool = schoolContext.id
    const scope = `${scopeKey()}:${session.user?.id ?? ''}`
    const dirty = invalidatedPosts(scope, activeSchool)
    const ids = new Set(posts.value.map(post => post.id))
    const queryChanged = dirty.some(item => item.content || (mode.value === 'following' && item.authorId) || (mode.value === 'collect' && !ids.has(item.id)))
    await (posts.value.length && !queryChanged
      ? run(syncInvalidated)
      : load(true, true, undefined, true))
    if (!error.value && activeSchool === schoolContext.id) recordPage('feed_open', activeSchool)
  },
  refresh: async () => { await Promise.all([load(), hotPosts.value?.refresh(), operations.value?.refresh(), shortcuts.value?.refresh()]) },
  loadMore: () => {
    if (more.value) return load(false)
  },
})
</script>
<template>
  <view class="page compact-page feed-page">
    <SchoolHeader v-if="homePage" :school="school" compact searchable @search="navigate('/pages/index/search')" />
    <HomeOperations v-if="homePage && !searchPage" ref="operations" />
    <HomeShortcuts v-if="homePage && !searchPage" ref="shortcuts" />
    <text v-if="session.user?.bound_school_id && schoolContext.loaded && !schoolContext.school">
      学校暂不可用，请联系管理员
    </text>
    <view v-if="!homePage && !lockedMode" class="row spread">
      <text class="heading">{{ school?.name || '校园社区' }}</text>
    </view>
    <HotPosts v-if="homePage" ref="hotPosts" :active="hotPostsActive" />
    <text v-if="lockedMode" class="heading">{{ ownPostsPage ? '我的发布' : '关注动态' }}</text>
    <scroll-view v-if="!lockedMode" scroll-x class="feed-filters" :show-scrollbar="false">
    <view class="filter-strip tabs">
      <button
        size="mini"
        :disabled="busy"
        :class="{ selected: mode === 'feed' }"
        @tap="setMode('feed')"
      >
        发现
      </button>
      <button
        size="mini"
        :disabled="busy"
        :class="{ selected: mode === 'following' }"
        @tap="setMode('following')"
      >
        关注
      </button>
      <button
        size="mini"
        :disabled="busy"
        :class="{ selected: mode === 'collect' }"
        @tap="setMode('collect')"
      >
        收藏
      </button>
      <button
        size="mini"
        :disabled="busy"
        :class="{ selected: mode === 'mine' }"
        @tap="navigate('/pages/index/search?view=mine')"
      >
        我的发布
      </button>
    </view>
    </scroll-view>
    <scroll-view v-if="!ownPostsPage && menu && (menu.settings.latest_visible || menu.tags.length)" scroll-x class="feed-filters" :show-scrollbar="false">
      <view class="filter-strip">
        <button v-if="menu.settings.latest_visible" size="mini" :disabled="busy" :class="{selected: !selectedTag}" @tap="selectTag('')">{{ menu.settings.latest_label }}</button>
        <button v-for="tag in menu.tags" :key="tag.id" size="mini" :disabled="busy" :class="{selected: selectedTag === tag.name}" @tap="selectTag(tag.name)"><text v-if="tag.menu_status === 'hot'" class="hot-tag">热</text>{{ tag.name }}</button>
      </view>
    </scroll-view>
    <view v-if="!homePage" class="row feed-search">
      <input
        v-model="search"
        :disabled="busy"
        class="input grow"
        placeholder="搜索校园内容"
        confirm-type="search"
        @confirm="load()"
      />
      <button size="mini" @tap="load()"><AppIcon name="search" :size="28" />搜索</button>
    </view>
    <view class="feed-list">
      <PageWindow :page="page" :busy="busy" @previous="target => load(false, true, target)" />
      <PostFeedCard v-for="post in posts" :key="post.id" :post="post" :school="schoolContext.id" :mine="mode === 'mine'" @open="detail" />
    </view>
    <PageStatus :busy="busy" :error="error" :empty="!posts.length" @retry="load()" />
    <button v-if="more" :disabled="busy" @tap="load(false)">{{ page % 3 === 0 ? '下一组' : '加载更多' }}</button>
  </view>
</template>
<style scoped>
.feed-list { display: flex; flex-direction: column; gap: var(--linjian-space-list); margin-top: 12rpx; }
.feed-filters { width: 100%; }
.filter-strip { display: flex; flex-wrap: nowrap; align-items: center; gap: 8rpx; }
.filter-strip button { flex-shrink: 0; min-height: 44px; margin: 0; padding: 0 16rpx; display: flex; align-items: center; line-height: 1.4; white-space: nowrap; }
.feed-search { flex-wrap: nowrap; }
.feed-search .input { width: auto; min-width: 0; }
.hot-tag { margin-right: 8rpx; color: #b45309; font-size: var(--linjian-font-caption); }
</style>
