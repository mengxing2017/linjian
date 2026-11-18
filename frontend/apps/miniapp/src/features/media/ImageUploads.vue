<script setup lang="ts">
import { ref, watch } from 'vue'
import { uploadImage } from './files'
import { uploadKey, type UploadPurpose } from './api'
import PrivateImage from './PrivateImage.vue'
import { schoolContext, requireSchool, scopeKey, onScopeChange } from '../school-context/context'
import { usePage } from '../../shared/use-page'
import { onUnmounted } from 'vue'
const props = defineProps<{ modelValue: string[]; purpose: UploadPurpose; disabled?: boolean }>()
const emit = defineEmits<{ 'update:modelValue': [value: string[]]; busy: [value: boolean] }>()
const failed = ref<{ path: string; key: string; message: string }[]>([])
const { busy, error, run } = usePage(() => { failed.value = []; emit('busy', false) })
watch(busy, v => emit('busy', v))
const stop = onScopeChange(() => { failed.value = [] })
onUnmounted(() => { stop(); emit('busy', false) })
async function send(item: { path: string; key: string }) {
  const scope = scopeKey(), school = schoolContext.id
  try {
    const v = await uploadImage(school, item.path, props.purpose, item.key)
    if (scope !== scopeKey()) return
    if (!props.modelValue.includes(v.url)) emit('update:modelValue', [...props.modelValue, v.url])
    failed.value = failed.value.filter(v => v.key !== item.key)
  } catch (e) {
    if (scope !== scopeKey()) return
    const message = e instanceof Error ? e.message : '上传失败'
    failed.value = [...failed.value.filter(v => v.key !== item.key), { ...item, message }]
  }
}
function choose() {
  void run(async () => {
    if (!(await requireSchool())) return
    const scope = scopeKey(), count = 9 - props.modelValue.length - failed.value.length
    if (count <= 0) throw new Error('最多9张图片，请先移除或重试失败项')
    const result = await uni.chooseImage({ count, sizeType: ['compressed'] })
    for (const path of result.tempFilePaths) {
      if (scope !== scopeKey()) return
      await send({ path, key: uploadKey() })
    }
  })
}
</script>
<template>
  <view class="image-grid">
    <view v-for="(src, i) in modelValue" :key="src">
      <PrivateImage :school="schoolContext.id" :source="src" />
      <button size="mini" :disabled="busy || disabled" @tap="emit('update:modelValue', modelValue.filter((_, n) => n !== i))">移除</button>
      <button v-if="i > 0" size="mini" :disabled="busy || disabled" @tap="emit('update:modelValue', [...modelValue.slice(0, i - 1), src, modelValue[i - 1]!, ...modelValue.slice(i + 1)])">前移</button>
    </view>
    <view v-for="item in failed" :key="item.key">
      <text>{{ item.message }}</text>
      <button size="mini" :disabled="busy || disabled" @tap="run(async () => { await send(item) })">重试此图片</button>
      <button size="mini" :disabled="busy || disabled" @tap="failed = failed.filter(v => v.key !== item.key)">移除失败项</button>
    </view>
  </view>
  <button :disabled="busy || disabled || modelValue.length + failed.length >= 9" @tap="choose">选择图片（{{ modelValue.length }}/9）</button>
  <text v-if="busy">图片上传中…</text>
  <text v-if="error">{{ error }}</text>
  <text class="muted">支持 PNG/JPEG/GIF/WebP，最大 5 MiB。上传后提交表单生效。</text>
</template>
