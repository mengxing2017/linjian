<script setup lang="ts">
import { ref } from 'vue'
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
const props = defineProps<{ id?: string; school?: string }>()
const phone = ref<InstanceType<typeof PhoneBindingPanel>>()
const ensurePhone = () => phone.value?.ensure() ?? Promise.resolve(false)
const titleExpanded = ref(false)
const title = ref(''),
  content = ref(''),
  tags = ref(''),
  images = ref<string[]>([]),
  revision = ref<number>(),
  loaded = ref(''),
  message = ref(''),
  candidates = ref<HomeMenuTag[]>([]),
  selectedCandidates = ref<HomeMenuTag[]>([])
const uploading = ref(false)
const pendingImages = ref(0)
const submitting = ref(false)
const { busy, error, run } = usePage(() => {
  title.value = ''
  titleExpanded.value = false
  content.value = ''
  tags.value = ''
  images.value = []
  revision.value = undefined
  loaded.value = ''
  message.value = ''
  candidates.value = []
  selectedCandidates.value = []
})
const draftKey = () => postDraftKey(session.user?.id, schoolContext.id, props.id)
const draftId = ref(uuid())
const leaseMessage = ref('')
function saveDraft() {
  if (
    !session.user ||
    !schoolContext.id ||
    (!title.value.trim() && !content.value.trim() && !images.value.length)
  )
    return
  uni.setStorageSync(draftKey(), {
    title: title.value,
    content: content.value,
    tags: tags.value,
    images: images.value,
    revision: revision.value,
    draft_id: draftId.value,
  })
  const scope = scopeKey()
  void keepDraftImages(schoolContext.id, draftId.value, images.value).then(v => { if (scope === scopeKey()) leaseMessage.value = `草稿图片保留 ${v.lease_days} 天；恢复时重新核验。` }).catch(() => { if (scope === scopeKey()) leaseMessage.value = '草稿已保存本机，图片保留未确认，恢复时可能需要重新上传。' })
}
const stop = onScopeChange(() => {
  loaded.value = ''
})
onUnmounted(stop)
function loadData() {
  return (async () => {
    if (!(await requireSchool(props.school))) return false
    const key = scopeKey()
    if (loaded.value === key) return true
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
      tags.value = post.tags.join(',')
      images.value = [...post.images]
      revision.value = post.revision
    }
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
      tags.value = typeof v.tags === 'string' ? v.tags : ''
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
    loaded.value = key
    return true
  })()
}
let loading: Promise<boolean> | undefined
function ensureLoaded() {
  if (loaded.value && loaded.value === scopeKey()) return Promise.resolve(true)
  if (!loading) loading = loadData().finally(() => { loading = undefined })
  return loading
}
function load() {
  return run(async () => {
    await ensureLoaded()
  })
}

function addTag(tag: HomeMenuTag) {
  const name = tag.name
  const current = tags.value.split(/[,，]/).map(v => v.trim()).filter(Boolean)
  if (!current.includes(name) && current.length < 10) { tags.value = [...current, name].join(','); selectedCandidates.value = [...selectedCandidates.value, tag] }
}

async function submit() {
  if (submitting.value || uploading.value) {
    uni.showToast({ title: '正在处理，请稍候', icon: 'none' })
    return
  }
  submitting.value = true
  error.value = ''
  try {
    if (!(await ensureLoaded())) throw new Error('请先选择学校并加载编辑页面')
    const normalized = normalizePostContent({ title: title.value, content: content.value })
    const s = schoolContext.id,
      key = scopeKey(),
      draft = draftKey()
    saveDraft()
    if (!await ensurePhone() || key !== scopeKey()) return
    if (pendingImages.value) throw new Error('还有未上传的图片，请重试或移除后再发布；正文已保存')
    const input = {
        ...normalized,
        images: [...images.value],
        tags: tags.value
          .split(/[,，]/)
          .map((v) => v.trim())
          .filter(Boolean),
        candidate_tag_ids: selectedCandidates.value.filter(tag => tags.value.split(/[,，]/).some(v => v.trim() === tag.name)).map(tag => tag.id),
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
    tags.value = ''
    images.value = []
    revision.value = result.revision
    uni.showModal({
      title: '提交成功',
      content: message.value,
      showCancel: false,
      success: () => { if (key === scopeKey()) uni.redirectTo({ url: `/pages/index/detail?school=${s}&id=${result.id}` }) },
    })
  } catch (e) {
    if (e && typeof e === 'object' && 'code' in e && e.code === 'phone_verification_required') void phone.value?.open()
    error.value = handlePageError(e)
    uni.showToast({ title: error.value.slice(0, 30), icon: 'none' })
  } finally {
    submitting.value = false
  }
}
function discard() {
  void run(async () => {
    if (await confirm('确认丢弃当前草稿？')) {
      uni.removeStorageSync(draftKey())
      title.value = ''
      titleExpanded.value = false
      content.value = ''
      tags.value = ''
      images.value = []
    }
  })
}
// Scope changes clear in-memory fields; drafts are explicitly saved under account + school.
defineExpose({ show: load, hide: () => { saveDraft(); phone.value?.close() } })
</script>
<template>
  <view class="page">
    <text class="heading">{{ props.id ? '编辑动态' : '分享校园生活' }}</text>
    <text class="muted">
      {{ schoolContext.schools.find((s) => s.id === schoolContext.id)?.name }}
    </text>
    <PageStatus :busy="busy" :error="error" @retry="load" />
    <view class="card">
      <view v-if="titleExpanded">
        <input v-model="title" class="input" maxlength="-1" placeholder="标题（选填，最多 200 字）" />
        <button size="mini" @tap="titleExpanded = false">收起标题</button>
        <button size="mini" @tap="title = ''; titleExpanded = false">移除标题</button>
      </view>
      <textarea
        v-model="content"
        class="input editor"
        maxlength="-1"
        placeholder="分享此刻的想法，或说说你想问的事……"
      />
      <button v-if="!titleExpanded" size="mini" @tap="titleExpanded = true">{{ title ? '展开标题' : '添加标题（选填）' }}</button>
      <view class="row"><text>可选标签：</text><button v-for="tag in candidates" :key="tag.id" size="mini" @tap="addTag(tag)">{{ tag.name }}</button></view>
      <input v-model="tags" class="input" placeholder="标签，使用逗号分隔" />
      <ImageUploads v-model="images" purpose="post_image" :before-upload="ensurePhone" :disabled="busy" @busy="uploading = $event" @pending="pendingImages = $event" />
      <text v-if="leaseMessage" class="muted">{{ leaseMessage }}</text>
      <view class="row">
        <button size="mini" :disabled="busy || uploading" @tap="saveDraft">保存本地草稿</button>
        <button size="mini" :disabled="busy || uploading" @tap="discard">丢弃草稿</button>
      </view>
      <text v-if="error" class="error">{{ error }}</text>
      <button class="primary" :disabled="submitting || uploading" :loading="submitting" @tap="submit">发布</button>
      <text class="muted">网络中断时请先在“我的发布”核对结果，再决定是否重新提交。</text>
      <text class="muted">首次发布需通过微信验证手机号；暂不验证仍可浏览和保存正文草稿。</text>
    </view>
    <PhoneBindingPanel ref="phone" />
  </view>
</template>
