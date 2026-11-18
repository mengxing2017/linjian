<script setup lang="ts">
import { ref, watch } from 'vue'

import ImageUploads from '../media/ImageUploads.vue'
import { uuid } from '../operations/policy'
import { feedbackApi, type FeedbackType } from './api'
import { requireSchool, schoolContext, scopeKey } from '../school-context/context'
import { usePage } from '../../shared/use-page'
import PageStatus from '../../shared/PageStatus.vue'
const feedbackType = ref<FeedbackType>(), submissionKey = ref(uuid())
const typeLabels = ['产品建议','bug反馈','其他建议']
const typeValues: FeedbackType[] = ['product_suggestion','bug_report','other']
const content = ref(''),
  contact = ref(''),
  submitted = ref(false),
  images = ref<string[]>([])
const uploading = ref(false)
const { busy, error, run } = usePage(() => {
  feedbackType.value = undefined
  submissionKey.value = uuid()
  content.value = ''
  contact.value = ''
  submitted.value = false
  images.value = []
})
watch([content,contact,feedbackType,images], () => {submissionKey.value=uuid()}, {deep:true})
function load() {
  return run(async () => {
    await requireSchool()
  })
}
function submit() {
  void run(async () => {
    if (!(await requireSchool())) return
    if (!feedbackType.value) throw new Error('请选择反馈类型')
    if (!contact.value.trim()) throw new Error('请填写联系方式')
    const key = scopeKey()
    if (!content.value.trim()) throw new Error('请填写反馈内容')
    await feedbackApi.submit(schoolContext.id, {feedback_type:feedbackType.value, content:content.value, contact:contact.value, images:[...images.value], submission_key:submissionKey.value})
    if (key !== scopeKey()) return
    submissionKey.value = uuid()
    feedbackType.value = undefined
    images.value = []
    content.value = ''
    contact.value = ''
    submitted.value = true
  })
}

defineExpose({ show: load })
</script>
<template>
  <view class="page">
    <text class="heading">投诉反馈</text>
    <PageStatus :busy="busy" :error="error" @retry="load" />
    <view class="card">
      <picker :range="typeLabels" @change="feedbackType=typeValues[Number($event.detail.value)]"><view class="input">{{ feedbackType ? typeLabels[typeValues.indexOf(feedbackType)] : '请选择反馈类型' }}</view></picker>
      <textarea
        v-model="content"
        class="input editor"
        maxlength="500"
        placeholder="请描述你的意见或遇到的问题"
      />
      <input v-model="contact" class="input" maxlength="120" placeholder="联系方式（必填）" />
      <ImageUploads v-model="images" purpose="feedback_evidence" :disabled="busy" @busy="uploading = $event" />
      <button class="primary" :disabled="busy || uploading" @tap="submit">提交反馈</button>
      <text v-if="submitted" class="muted">已提交，感谢你的反馈。</text>
    </view>
  </view>
</template>
