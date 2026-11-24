<script setup lang="ts">
import { computed, ref } from 'vue'
import AppIcon from '../../shared/AppIcon.vue'
import { invalidatePost } from './post-invalidations'
import { postDraftKey } from './post-draft'
import { communityApi, homeMenuApi, normalizePostContent, type HomeMenuTag, type Post } from './api'
import { schoolContext, requireSchool, scopeKey, onScopeChange } from '../school-context/context'
import { session } from '../identity/session'
import { keepDraftImages } from '../media/files'
import { uuid } from '../operations/policy'
import ImageUploads from '../media/ImageUploads.vue'
import { usePage, confirm, handlePageError } from '../../shared/use-page'
import PageStatus from '../../shared/PageStatus.vue'
import PhoneBindingPanel from '../miniapp-users/PhoneBindingPanel.vue'
import { onUnmounted } from 'vue'
import { createPublishExit } from './publish-exit'
import { leavePublish, finishPublish } from '../../app/navigation'
import { clearReturnTarget } from '../identity/return-target'
import PublishTagPicker from './PublishTagPicker.vue'
import { emptyPublishTags, matchPublishTags, restorePublishTags, publishTagNames, publishTagInput, publishTagDraft, type PublishTags } from './publish-tags'
const props = defineProps<{ id?: string; school?: string }>()
const phone = ref<InstanceType<typeof PhoneBindingPanel>>()
const ensurePhone = () => phone.value?.ensure() ?? Promise.resolve(false)
const titleExpanded = ref(false)
const title = ref(''),
  content = ref(''),
  tags = ref(emptyPublishTags()),
  images = ref<string[]>([]),
  revision = ref<number>(),
  loaded = ref(''),
  message = ref(''),
  candidates = ref<HomeMenuTag[]>([])
const originalTags = ref<string[]>([])
const pickerOpen = ref(false)
const tagConfirmationRequired = ref(false)
let pickerScope = ''
const tagNames = computed(() => publishTagNames(tags.value))
const tagSummary = computed(() => tagNames.value.slice(0, 3).join(' · '))
const uploading = ref(false)
const pendingImages = ref(0)
const submitting = ref(false)
let visible = false
let draftScope: { key: string; school: string; account: string; scope: string } | undefined
const { busy, error, run } = usePage(() => {
  draftScope = undefined
  title.value = ''
  titleExpanded.value = false
  content.value = ''
  tags.value = emptyPublishTags()
  originalTags.value = []
  pickerOpen.value = false
  tagConfirmationRequired.value = false
  images.value = []
  revision.value = undefined
  loaded.value = ''
  message.value = ''
  candidates.value = []
  leaseMessage.value = ''
})
const draftKey = () => postDraftKey(session.user?.id, schoolContext.id, props.id)
const draftId = ref(uuid())
const leaseMessage = ref('')
function saveDraft() {
  if (!draftScope || draftScope.scope !== scopeKey() || draftScope.account !== session.user?.id) return false
  if (!session.user || !schoolContext.id) return false
  try {
    if (!title.value.trim() && !content.value.trim() && !tagNames.value.length && !tags.value.unresolved.length && !images.value.length) {
      uni.removeStorageSync(draftScope.key)
      return true
    }
    uni.setStorageSync(draftScope.key, {
      title: title.value,
      content: content.value,
      ...publishTagDraft(tags.value),
      tags_need_confirmation: tagConfirmationRequired.value,
      images: images.value,
      revision: revision.value,
      draft_id: draftId.value,
    })
  } catch {
    uni.showToast({ title: '草稿保存失败，请勿退出', icon: 'none' })
    return false
  }
  const scope = scopeKey()
  void keepDraftImages(draftScope.school, draftId.value, images.value).then(v => { if (scope === scopeKey()) leaseMessage.value = `草稿图片保留 ${v.lease_days} 天；恢复时重新核验。` }).catch(() => { if (scope === scopeKey()) leaseMessage.value = '草稿已保存本机，图片保留未确认，恢复时可能需要重新上传。' })
  return true
}
function clearDraft() {
  if (draftScope?.scope === scopeKey()) uni.removeStorageSync(draftScope.key)
  title.value = ''
  titleExpanded.value = false
  content.value = ''
  tags.value = emptyPublishTags()
  pickerOpen.value = false
  tagConfirmationRequired.value = false
  images.value = []
}
const exit = createPublishExit({
  snapshot: () => JSON.stringify([title.value, content.value, tags.value, tagConfirmationRequired.value, images.value, pendingImages.value]),
  save: saveDraft,
  discard: clearDraft,
  busy: () => busy.value || uploading.value || submitting.value,
  scope: scopeKey,
  leave: () => props.id ? uni.navigateBack({ delta: 1 }) : leavePublish(),
})
const stop = onScopeChange(() => {
  loaded.value = ''
  loading = undefined
  pickerOpen.value = false
})
onUnmounted(stop)
function loadData() {
  return (async () => {
    if (!(await requireSchool(props.school))) return false
    const key = scopeKey()
    if (loaded.value === key) return true
    if (!session.user) return false
    draftScope = { key: draftKey(), school: schoolContext.id, account: session.user.id, scope: key }
    const menu = await homeMenuApi.public(schoolContext.id)
    if (key !== scopeKey()) return false
    candidates.value = menu.tags
    let post: Post | undefined
    if (props.id) post = await communityApi.post(schoolContext.id, props.id)
    if (key !== scopeKey()) return false
    if (post) {
      if (post.author_id !== session.user?.id) throw new Error('只能编辑自己的动态')
      title.value = post.title
      titleExpanded.value = !!post.title
      content.value = post.content
      originalTags.value = [...post.tags]
      tags.value = matchPublishTags(post.tags, candidates.value, originalTags.value)
      images.value = [...post.images]
      revision.value = post.revision
    }
    exit.checkpoint()
    const draft: unknown = uni.getStorageSync(draftKey())
    if (
      draft &&
      typeof draft === 'object' &&
      (await confirm('发现当前账号在本校的草稿，是否恢复？'))
    ) {
      if (key !== scopeKey()) return false
      const v = draft as Record<string, unknown>
      if (typeof v.draft_id === 'string') draftId.value = v.draft_id
      title.value = typeof v.title === 'string' ? v.title : ''
      titleExpanded.value = !!title.value
      content.value = typeof v.content === 'string' ? v.content : ''
      tags.value = restorePublishTags(v, candidates.value, originalTags.value)
      tagConfirmationRequired.value = v.tags_need_confirmation === true
      images.value = Array.isArray(v.images)
        ? v.images.filter(
            (x): x is string =>
              typeof x === 'string' && x.startsWith(`/api/v1/schools/${schoolContext.id}/media/`),
          )
        : []
      // An edit draft must still refer to the current revision. Conflicts require a fresh read.
      if (post && v.revision !== post.revision) {
        images.value = [...post.images]
        throw new Error('帖子已更新，请核对恢复的正文后再提交')
      }
    }
    if (images.value.length) {
      try { await keepDraftImages(schoolContext.id, draftId.value, images.value) }
      catch { leaseMessage.value = '部分草稿图片已失效，请移除失效图片并重新上传。' }
    }
    if (key !== scopeKey()) return false
    loaded.value = key
    if (!props.id) clearReturnTarget()
    return true
  })()
}
let loading: Promise<boolean> | undefined
function ensureLoaded() {
  if (loaded.value && loaded.value === scopeKey()) return Promise.resolve(true)
  if (!loading) {
    const operation = loadData().finally(() => { if (loading === operation) loading = undefined })
    loading = operation
  }
  return loading
}
function load() {
  visible = true
  return run(async () => {
    await ensureLoaded()
  })
}

function openTags() {
  if (busy.value || submitting.value || uploading.value) return
  uni.hideKeyboard()
  pickerScope = scopeKey()
  pickerOpen.value = true
}
function confirmTags(value: PublishTags) {
  if (!pickerOpen.value || pickerScope !== scopeKey() || busy.value || submitting.value) return
  tags.value = value
  tagConfirmationRequired.value = false
  pickerOpen.value = false
}
function cancel() {
  if (pickerOpen.value) pickerOpen.value = false
  else void exit.cancel()
}
function inputTitle(event: unknown) {
  title.value = (event as { detail: { value: string } }).detail.value
}
function inputContent(event: unknown) {
  content.value = (event as { detail: { value: string } }).detail.value
}

async function submit() {
  if (busy.value || submitting.value || uploading.value || pickerOpen.value) {
    uni.showToast({ title: '正在处理，请稍候', icon: 'none' })
    return
  }
  submitting.value = true
  error.value = ''
  let submissionScope = scopeKey()
  try {
    if (!(await ensureLoaded())) throw new Error('请先选择学校并加载编辑页面')
    submissionScope = scopeKey()
    const normalized = normalizePostContent({ title: title.value, content: content.value })
    const s = schoolContext.id,
      key = scopeKey(),
      draft = draftKey()
    if (!saveDraft()) return
    if (tagConfirmationRequired.value) throw new Error('标签配置已变化，请打开标签面板确认后提交')
    const tagInput = publishTagInput(tags.value)
    if (!await ensurePhone() || key !== scopeKey()) return
    if (pendingImages.value) throw new Error('还有未上传的图片，请重试或移除后再发布；正文已保存')
    const input = {
        ...normalized,
        images: [...images.value],
        ...tagInput,
      }
    if (props.id && !revision.value) throw new Error('请重新加载帖子后编辑')
    const result = props.id
      ? await communityApi.update(s, props.id, { ...input, revision: revision.value! })
      : await communityApi.create(s, input)
    if (key !== scopeKey()) return
    uni.removeStorageSync(draft)
    invalidatePost(`${scopeKey()}:${session.user?.id ?? ''}`, s, result.id, true)
    message.value = result.audit_status === 'approved' ? '发布成功' : '已提交，等待审核'
    title.value = ''
    titleExpanded.value = false
    content.value = ''
    tags.value = emptyPublishTags()
    images.value = []
    revision.value = result.revision
    exit.checkpoint()
    if (!visible) return
    uni.showModal({
      title: '提交成功',
      content: message.value,
      showCancel: false,
      success: () => {
        if (key !== scopeKey() || !visible) return
        const operation = props.id
          ? uni.redirectTo({ url: `/pages/index/detail?school=${s}&id=${result.id}` })
          : finishPublish(s, result.id)
        void operation.catch(() => uni.showToast({ title: '提交已成功，可在我的发布中查看', icon: 'none' }))
      },
    })
  } catch (e) {
    if (submissionScope !== scopeKey()) return
    if (visible && e && typeof e === 'object' && 'code' in e && e.code === 'phone_verification_required') void phone.value?.open()
    error.value = handlePageError(e)
    uni.showToast({ title: error.value.slice(0, 30), icon: 'none' })
    if (e && typeof e === 'object' && 'code' in e && e.code === 'validation' && error.value.includes('标签')) {
      try {
        const menu = await homeMenuApi.public(schoolContext.id)
        if (submissionScope !== scopeKey()) return
        candidates.value = menu.tags
        tags.value = restorePublishTags(publishTagDraft(tags.value), menu.tags, originalTags.value)
        tagConfirmationRequired.value = true
        error.value += '；候选已刷新，请重新确认标签后提交'
      } catch {
        if (submissionScope === scopeKey()) error.value += '；候选刷新失败，请稍后重试'
      }
    }
  } finally {
    submitting.value = false
  }
}
function discard() {
  void run(async (current) => {
    if (await confirm('确认丢弃当前草稿？') && current()) {
      clearDraft()
      exit.checkpoint()
    }
  })
}
// Scope changes clear in-memory fields; drafts are explicitly saved under account + school.
defineExpose({ show: load, hide: () => { visible = false; pickerOpen.value = false; exit.saveOnHide(); phone.value?.close() }, cancel })
</script>
<template>
  <view class="page">
    <view class="row spread">
      <text class="heading">{{ props.id ? '编辑动态' : '分享校园生活' }}</text>
      <button size="mini" :disabled="busy || uploading || submitting" @tap="cancel">取消</button>
    </view>
    <text class="muted">
      {{ schoolContext.schools.find((s) => s.id === schoolContext.id)?.name }}
    </text>
    <PageStatus :busy="busy" :error="error" @retry="load" />
    <view class="card">
      <view v-if="titleExpanded">
        <input :value="title" :disabled="busy || submitting || pickerOpen" class="input" maxlength="-1" placeholder="标题（选填，最多 200 字）" @input="inputTitle" />
        <button size="mini" :disabled="busy || submitting" @tap="titleExpanded = false">收起标题</button>
        <button size="mini" :disabled="busy || submitting" @tap="title = ''; titleExpanded = false">移除标题</button>
      </view>
      <textarea
        :value="content"
        :disabled="busy || submitting || pickerOpen"
        class="input editor"
        maxlength="-1"
        placeholder="分享此刻的想法，或说说你想问的事……"
        @input="inputContent"
      />
      <button v-if="!titleExpanded" size="mini" :disabled="busy || submitting" @tap="titleExpanded = true">{{ title ? '展开标题' : '添加标题（选填）' }}</button>
      <button class="publish-tag-summary" :disabled="busy || submitting || uploading || (!candidates.length && !tagNames.length && !tags.unresolved.length && !tagConfirmationRequired)" aria-label="选择标签" @tap="openTags">
        <text>标签（选填）</text>
        <text class="publish-tag-copy">{{ tagSummary || (tags.unresolved.length ? '有待处理的旧草稿标签' : candidates.length ? '选择标签' : '暂无可选标签') }}</text>
        <text v-if="tagNames.length" class="publish-tag-count">已选 {{ tagNames.length }}</text><text>›</text>
      </button>
      <text v-if="tags.unresolved.length" class="error">有 {{ tags.unresolved.length }} 个旧草稿标签待处理，请打开标签面板确认移除。</text>
      <text v-if="tagConfirmationRequired" class="error">标签配置已变化，请打开标签面板重新确认。</text>
      <ImageUploads v-model="images" purpose="post_image" :before-upload="ensurePhone" :disabled="busy || submitting" @busy="uploading = $event" @pending="pendingImages = $event" />
      <text v-if="leaseMessage" class="muted">{{ leaseMessage }}</text>
      <view class="row">
        <button size="mini" :disabled="busy || uploading || submitting" @tap="saveDraft">保存本地草稿</button>
        <button size="mini" :disabled="busy || uploading || submitting" @tap="discard">丢弃草稿</button>
      </view>
      <text v-if="error" class="error">{{ error }}</text>
      <button class="primary" :disabled="busy || submitting || uploading" :loading="submitting" @tap="submit"><AppIcon name="send" :size="32" />发布</button>
      <text class="muted">网络中断时请先在“我的发布”核对结果，再决定是否重新提交。</text>
      <text class="muted">首次发布需通过微信验证手机号；暂不验证仍可浏览和保存正文草稿。</text>
      <text class="muted">点击“取消”可选择保存或放弃；使用系统返回时自动保存本地草稿，草稿图片恢复时会重新核验。</text>
    </view>
    <PhoneBindingPanel ref="phone" />
    <PublishTagPicker :open="pickerOpen" :candidates="candidates" :selection="tags" :disabled="busy || submitting" @close="pickerOpen = false" @confirm="confirmTags" />
  </view>
</template>
<style scoped>
.publish-tag-summary { display: flex; align-items: center; gap: 16rpx; width: 100%; min-height: 88rpx; margin: 16rpx 0; font-size: 28rpx; text-align: left; }
.publish-tag-copy { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.publish-tag-count { flex-shrink: 0; font-size: 24rpx; }
</style>
