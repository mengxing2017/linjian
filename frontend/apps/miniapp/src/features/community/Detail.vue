<script setup lang="ts">
import PageWindow from '../../shared/PageWindow.vue'
import AppIcon from '../../shared/AppIcon.vue'
import { onUnmounted, ref, watch, nextTick } from 'vue'
import { recordPage } from '../statistics/activity'
import { postVisibilityLabel } from './post-status'
import { communityApi, type Comment } from './api'
import { requireSchool, scopeKey, onScopeChange } from '../school-context/context'
import { session, restoreSession } from '../identity/session'
import { saveReturnTarget, validPostTarget } from '../identity/return-target'
import { confirm, navigate, handlePageError } from '../../shared/use-page'
import { createPostDetail } from './use-post-detail'
import { postShare } from './sharing'
import { invalidatePost, invalidatedPosts, clearPostInvalidations } from './post-invalidations'
import PrivateImage from '../media/PrivateImage.vue'
import PageStatus from '../../shared/PageStatus.vue'
import PhoneBindingPanel from '../miniapp-users/PhoneBindingPanel.vue'

const props = defineProps<{ id: string; school: string }>()
const phone = ref<InstanceType<typeof PhoneBindingPanel>>()
const invalidationScope = () => `${scopeKey()}:${session.user?.id ?? ''}`
const key = () => `${invalidationScope()}:${props.school}:${props.id}`
const notify = (title: string) => uni.showToast({ title, icon: 'none' })
const detail = createPostDetail({
  api: communityApi, school: () => props.school, id: () => props.id, key,
  invalidate: kind => invalidatePost(invalidationScope(), props.school, props.id, false, kind === 'follow' ? post.value?.author_id : undefined),
  failure: cause => {
    if (cause && typeof cause === 'object' && 'code' in cause && cause.code === 'phone_verification_required') void phone.value?.open()
    return handlePageError(cause)
  }, notify, confirm,
  beforePublicAction: () => phone.value?.ensure() ?? Promise.resolve(false),
})
const { post, interaction, comments, content, parent, error, busy, loadingMore, operation, more, canInteract, canShare, page } = detail
const focused = ref(false)
const reason = ref('')
const auxiliaryBusy = ref(false)
let visible = false
let displayEpoch = 0
let contentVersion = 0
function hideMenu() {
  if (typeof uni.hideShareMenu === 'function') uni.hideShareMenu({ hideShareItems: ['shareAppMessage', 'shareTimeline'] })
}
watch(canShare, allowed => {
  hideMenu()
  if (allowed && typeof uni.showShareMenu === 'function') uni.showShareMenu({ menus: ['shareAppMessage'] })
}, { flush: 'sync' })
function clear() {
  hideMenu()
  detail.reset()
  reason.value = ''
  focused.value = false
  auxiliaryBusy.value = false
  contentVersion = 0
}
const stop = onScopeChange(() => { clearPostInvalidations(); clear() })
watch(() => [props.school, props.id], clear, { flush: 'sync' })
onUnmounted(() => { visible = false; displayEpoch++; hideMenu(); stop(); detail.dispose() })
async function prepare() {
  const school = props.school
  const id = props.id
  if (!validPostTarget(school, id)) throw new Error('帖子链接无效')
  await restoreSession()
  if (school !== props.school || id !== props.id || !visible) return false
  if (!session.user?.bound_school_id) {
    saveReturnTarget(school, id)
    uni.navigateTo({ url: '/pages/user/login' })
    return false
  }
  const allowed = await requireSchool(school)
  return allowed && visible && school === props.school && id === props.id
}
async function load(full = false) {
  const turn = ++displayEpoch
  hideMenu()
  detail.show()
  try {
    if (!(await prepare()) || turn !== displayEpoch) return
    const target = key()
    const dirty = invalidatedPosts(invalidationScope(), props.school).find(item => item.id === props.id)
    const reload = full || !post.value || (dirty?.content && dirty.contentVersion > contentVersion)
    if (reload) await detail.load()
    else await detail.resume()
    if (turn !== displayEpoch || target !== key() || !visible) return
    if (!error.value && post.value && reload) {
      contentVersion = dirty?.contentVersion ?? contentVersion
      recordPage('post_open', props.school, props.id)
    }
  } catch (cause) {
    if (turn === displayEpoch && visible) {
      detail.reset()
      const message = handlePageError(cause)
      if (turn === displayEpoch && visible) error.value = message
    }
  }
}
async function focusComment(item?: Comment) {
  if (!canInteract.value) return
  if (item) detail.reply(item)
  focused.value = false
  uni.pageScrollTo({ selector: '#post-comment-input', duration: 200 })
  await nextTick()
  if (canInteract.value) focused.value = true
}
async function remove() {
  if (!post.value || auxiliaryBusy.value || busy.value) return
  const target = key()
  const item = post.value
  if (!(await confirm('确认删除这条动态？')) || target !== key() || !visible) return
  auxiliaryBusy.value = true
  try {
    await communityApi.remove(props.school, item)
    if (target !== key()) return
    invalidatePost(invalidationScope(), props.school, props.id, true)
    detail.reset()
    if (visible) uni.navigateBack()
  } catch (cause) {
    if (target === key()) { const message = handlePageError(cause); if (target === key()) error.value = message }
  } finally { if (target === key()) auxiliaryBusy.value = false }
}
async function report() {
  if (auxiliaryBusy.value || busy.value) return
  if (!reason.value.trim()) { error.value = '请填写举报原因'; return }
  const target = key()
  const draft = reason.value
  auxiliaryBusy.value = true
  try {
    await communityApi.report(props.school, props.id, draft)
    if (target !== key()) return
    if (reason.value === draft) reason.value = ''
    if (visible) notify('已提交举报')
  } catch (cause) { if (target === key()) { const message = handlePageError(cause); if (target === key()) error.value = message } } finally {
    if (target === key()) auxiliaryBusy.value = false
  }
}
defineExpose({
  show: () => { visible = true; return load() },
  hide: () => { visible = false; displayEpoch++; detail.hide(); hideMenu(); phone.value?.close() },
  refresh: () => load(true),
  loadMore: detail.next,
  share: () => postShare(interaction.value, props.school, props.id, canShare.value),
})
</script>
<template>
  <PhoneBindingPanel ref="phone" />
  <view class="page compact-page detail-page">
    <PageStatus :busy="busy" :error="error" @retry="load(true)" />
    <template v-if="post">
      <view class="card">
        <text v-if="post.title" class="heading">{{ post.title }}</text>
        <text class="muted">{{ post.author_name || '校园成员' }}</text>
        <text v-if="post.visibility !== 'visible'" class="muted">状态：{{ postVisibilityLabel(post.visibility) }}</text>
        <text class="body">{{ post.content }}</text>
        <PrivateImage v-for="src in post.images" :key="src" :school="props.school" :source="src" />
        <view class="row"><text v-for="tag in post.tags" :key="tag" class="tag">#{{ tag }}</text></view>
        <text class="muted">{{ post.audit_status === 'approved' ? '已通过审核' : post.audit_status === 'pending' ? '待审核' : '未通过审核' }}</text>
        <view class="row post-actions">
          <button size="mini" :disabled="busy || !canInteract" @tap="detail.react('like')"><AppIcon name="like" :selected="!!interaction?.liked" :size="28" />{{ interaction?.liked ? '取消点赞' : '点赞' }} {{ post.like_count }}</button>
          <button size="mini" :disabled="!canInteract" @tap="focusComment()"><AppIcon name="comment" :size="28" />评论 {{ post.comment_count }}</button>
          <button v-if="canShare" size="mini" open-type="share">分享给好友/群</button>
          <button v-else size="mini" disabled>暂不可分享</button>
        </view>
        <text v-if="!canInteract" class="muted">当前帖子或账号状态不支持互动，请刷新核对。</text>
        <view class="row">
          <button size="mini" :disabled="busy || !canInteract" @tap="detail.react('collect')"><AppIcon name="collection" :selected="!!interaction?.collected" :size="28" />{{ interaction?.collected ? '取消收藏' : '收藏' }}</button>
          <button v-if="post.author_id !== session.user?.id" size="mini" :disabled="busy || !canInteract" @tap="detail.follow()"><AppIcon name="attention" :selected="!!interaction?.author_followed" :size="28" />{{ interaction?.author_followed ? '取消关注' : '关注作者' }}</button>
          <button v-if="post.author_id === session.user?.id" size="mini" :disabled="busy" @tap="navigate(`/pages/community/edit?school=${props.school}&id=${props.id}`)"><AppIcon name="posts" :size="28" />编辑</button>
          <button v-if="post.author_id === session.user?.id" size="mini" :disabled="busy || auxiliaryBusy" @tap="remove"><AppIcon name="delete" :size="28" />删除</button>
        </view>
      </view>
      <view class="card">
        <text class="title">评论 {{ post.comment_count }}</text>
        <view id="post-comments-anchor" />
        <PageWindow :page="page" :busy="busy || loadingMore" anchor="#post-comments-anchor" @previous="detail.loadComments" />
        <view v-for="item in comments" :key="item.id" class="comment">
          <text>{{ item.author_name }}：{{ item.content }}</text>
          <text v-if="item.parent_id" class="muted">回复评论</text>
          <text v-if="item.audit_status !== 'approved'" class="muted">{{ item.audit_status === 'pending' ? '待审核' : '未通过审核' }}</text>
          <view class="row">
            <button v-if="item.audit_status === 'approved' && item.visibility === 'visible' && !item.deleted" size="mini" :disabled="busy || !canInteract" @tap="focusComment(item)">回复</button>
            <button v-if="item.author_id === session.user?.id" size="mini" :disabled="busy || !canInteract" @tap="detail.removeComment(item)">删除</button>
          </view>
        </view>
        <button v-if="more" :disabled="busy || loadingMore" @tap="detail.next()">{{ page % 3 === 0 ? '下一组评论' : '更多评论' }}</button>
        <text v-if="parent" class="muted" @tap="parent = undefined">正在回复 {{ parent.authorName }}，点击取消</text>
        <textarea id="post-comment-input" v-model="content" :focus="focused" :disabled="!canInteract || operation === 'comment'" class="input" maxlength="2000" placeholder="友善交流，留下你的评论" @blur="focused = false" />
        <button class="primary" :disabled="busy || !canInteract" @tap="detail.comment()">提交评论</button>
      </view>
      <view class="card">
        <input v-model="reason" class="input" placeholder="举报原因" maxlength="500" />
        <button size="mini" :disabled="busy || auxiliaryBusy" @tap="report">举报动态</button>
      </view>
    </template>
  </view>
</template>
<style scoped>
.post-actions { padding-bottom: env(safe-area-inset-bottom); flex-wrap: wrap; }
.detail-page button[size='mini'] { min-height: 44px; min-width: 44px; display: inline-flex; align-items: center; justify-content: center; line-height: 1.4; }
.detail-page .row { gap: 8rpx; }
.detail-page .comment { font-size: var(--linjian-font-body); }
</style>
