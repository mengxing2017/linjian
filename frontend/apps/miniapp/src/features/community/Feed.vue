<script setup lang="ts">
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
import { postVisibilityLabel } from './post-status'
import { shallowRef, ref, computed, watch, onUnmounted } from 'vue'
import { communityApi, homeMenuApi, type HomeMenu, type CommunityPost, type FeedFilter } from './api'
import { requireSchool, schoolContext, scopeKey } from '../school-context/context'
import { invalidatedPosts, acknowledgePost, clearPostInvalidations } from './post-invalidations'
import { session } from '../identity/session'
import { usePage, navigate } from '../../shared/use-page'
import PageStatus from '../../shared/PageStatus.vue'
import PrivateImage from '../media/PrivateImage.vue'
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
const school = computed(() => schoolContext.schools.find((s) => s.id === schoolContext.id))
const hotPostsActive = computed(() => pageVisible.value && !!session.user && !!school.value && isHotPostsHome(!!props.homePage, !!props.searchPage, mode.value, search.value, selectedTag.value))
const { busy, error, run } = usePage(() => {
  clearPostInvalidations()
  posts.value = []
  page.value = 1
  more.value = false
  menu.value = undefined
  selectedTag.value = ''
})
function load(reset = true, retryAfterScopeRefresh = true): Promise<void> {
  return run(async (current) => {
    if (!(await requireSchool())) return
    if (!current()) {
      if (retryAfterScopeRefresh) void load(true, false)
      return
    }
    const id = schoolContext.id,
      generation = schoolContext.generation,
      user = session.user?.id
    const scope = `${scopeKey()}:${user ?? ''}`
    const dirty = invalidatedPosts(scope, id)
    if (reset) {
      const loaded = await homeMenuApi.public(id)
      if (!current() || generation !== schoolContext.generation || id !== schoolContext.id || user !== session.user?.id) return
      menu.value = loaded
      if (selectedTag.value && !loaded.tags.some(tag => tag.name === selectedTag.value)) selectedTag.value = ''
    }
    const next = reset ? 1 : page.value + 1
    const kind: FeedFilter['kind'] =
      mode.value === 'collect' || mode.value === 'following' ? mode.value : undefined
    const response = await communityApi.posts(id, {
      page: next,
      search: search.value,
      mine: mode.value === 'mine',
      kind,
      tag: selectedTag.value,
    })
    if (
      current() &&
      generation === schoolContext.generation &&
      id === schoolContext.id &&
      user === session.user?.id
    ) {
      posts.value = reset ? response.items : [...posts.value, ...response.items]
      page.value = next
      more.value = !!response.page?.has_more
      if (reset) dirty.forEach(item => acknowledgePost(scope, id, item.id, item.version))
    }
  })
}
function setMode(value: typeof mode.value) {
  if (busy.value) return
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
  for (const dirty of invalidatedPosts(scope, school)) {
    if (!posts.value.some(item => item.id === dirty.id)) continue
    try {
      const state = await communityApi.interactionState(school, dirty.id)
      if (!current() || scope !== `${scopeKey()}:${session.user?.id ?? ''}`) return
      const readable = mode.value === 'mine' || (state.audit_status === 'approved' && state.visibility === 'visible')
      const matches = readable && (mode.value !== 'collect' || state.collected) && (mode.value !== 'following' || state.author_followed)
      posts.value = posts.value.flatMap(item => item.id !== dirty.id ? [item] : matches ? [{ ...item, audit_status: state.audit_status, visibility: state.visibility, like_count: state.like_count, comment_count: state.comment_count }] : [])
      acknowledgePost(scope, school, dirty.id, dirty.version)
    } catch (cause) {
      if (!current() || scope !== `${scopeKey()}:${session.user?.id ?? ''}`) return
      if (cause && typeof cause === 'object' && 'status' in cause && cause.status === 404) {
        posts.value = posts.value.filter(item => item.id !== dirty.id)
        acknowledgePost(scope, school, dirty.id, dirty.version)
      } else throw cause
    }
  }
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
    const queryChanged = dirty.some(item => item.content || (mode.value === 'following' && item.authorId) || (mode.value === 'collect' && !posts.value.some(post => post.id === item.id)))
    await (posts.value.length && !queryChanged
      ? run(syncInvalidated)
      : load())
    if (!error.value && activeSchool === schoolContext.id) recordPage('feed_open', activeSchool)
  },
  refresh: async () => { await Promise.all([load(), hotPosts.value?.refresh(), operations.value?.refresh(), shortcuts.value?.refresh()]) },
  loadMore: () => {
    if (more.value) return load(false)
  },
})
</script>
<template>
  <view class="page">
    <HomeOperations v-if="!searchPage" ref="operations" />
    <SchoolHeader :school="school" />
    <HomeShortcuts v-if="!searchPage" ref="shortcuts" />
    <text v-if="session.user?.bound_school_id && !schoolContext.schools.length">
      学校暂不可用，请联系管理员
    </text>
    <view class="row spread">
      <text class="heading">{{ school?.name || '校园社区' }}</text>
      <button size="mini" @tap="navigate('/pages/plugin/index')">选校</button>
    </view>
    <HotPosts v-if="homePage" ref="hotPosts" :active="hotPostsActive" />
    <text v-if="lockedMode" class="heading">关注动态</text>
    <view v-if="!lockedMode" class="row tabs">
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
        @tap="setMode('mine')"
      >
        我的发布
      </button>
    </view>
    <scroll-view v-if="menu" scroll-x class="row">
      <button v-if="menu.settings.latest_visible" size="mini" :class="{selected: !selectedTag}" @tap="selectTag('')">{{ menu.settings.latest_label }}</button>
      <button v-for="tag in menu.tags" :key="tag.id" size="mini" :class="{selected: selectedTag === tag.name}" @tap="selectTag(tag.name)">{{ tag.name }}</button>
    </scroll-view>
    <view v-if="menu?.tags.some(tag => tag.menu_status === 'hot')" class="row">
      <text>热门：</text><button v-for="tag in menu.tags.filter(tag => tag.menu_status === 'hot')" :key="tag.id" size="mini" @tap="selectTag(tag.name)">{{ tag.name }}</button>
    </view>
    <view class="row">
      <input
        v-model="search"
        :disabled="busy"
        class="input grow"
        placeholder="搜索校园内容"
        confirm-type="search"
        @confirm="load()"
      />
      <button size="mini" @tap="load()">搜索</button>
    </view>
    <view class="waterfall">
      <view v-for="post in posts" :key="post.id" class="card post-card" @tap="detail(post)">
        <text v-if="post.title" class="title">{{ post.title }}</text>
        <text class="summary">{{ post.content }}</text>
        <text class="muted">查看全文</text>
        <PrivateImage v-if="post.images[0]" :school="schoolContext.id" :source="post.images[0]" />
        <text class="muted">发布时间：{{ post.published_at_text }}</text>
        <view class="row spread muted">
          <text>{{ post.author_name || '校园成员' }}</text>
          <text>♡ {{ post.like_count }} · 评论 {{ post.comment_count }}</text>
        </view>
        <text v-if="mode === 'mine'" class="muted">状态：{{ postVisibilityLabel(post.visibility) }}</text>
        <text v-if="mode === 'mine'" class="muted">
          {{
            post.audit_status === 'pending'
              ? '待审核'
              : post.audit_status === 'rejected'
                ? '未通过审核'
                : '已通过审核'
          }}
        </text>
      </view>
    </view>
    <PageStatus :busy="busy" :error="error" :empty="!posts.length" @retry="load()" />
    <button v-if="more" :disabled="busy" @tap="load(false)">加载更多</button>
  </view>
</template>
