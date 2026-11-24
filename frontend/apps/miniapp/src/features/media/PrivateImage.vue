<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, getCurrentInstance } from 'vue'
import { imagePath, releaseImages } from './files'
import { onScopeChange, clearScope } from '../school-context/context'
const props = withDefaults(defineProps<{ school: string; source: string; variant?: 'detail' | 'feed' }>(), { variant: 'detail' })
const local = ref(''),
  error = ref(false)
let generation = 0
const owner = {}
const instance = getCurrentInstance()
let mounted = false
let observer: ReturnType<typeof uni.createIntersectionObserver> | undefined
function disconnect() { observer?.disconnect(); observer = undefined }
const stop = onScopeChange(() => {
  generation++
  disconnect()
  releaseImages(owner)
  local.value = ''
  error.value = false
})
onUnmounted(() => {
  generation++
  disconnect()
  releaseImages(owner)
  stop()
})
async function load() {
  disconnect()
  const turn = ++generation
  local.value = ''
  error.value = false
  try {
    const path = await imagePath(props.school, props.source, owner)
    if (turn === generation) local.value = path
  } catch (failure) {
    if (turn === generation) {
      if (failure && typeof failure === 'object' && 'status' in failure && failure.status === 403)
        clearScope()
      error.value = true
    }
  }
}
function schedule() {
  generation++
  disconnect()
  releaseImages(owner)
  local.value = ''
  error.value = false
  if (!mounted) return
  if (props.variant !== 'feed' || !instance?.proxy || typeof uni.createIntersectionObserver !== 'function') {
    void load()
    return
  }
  const turn = generation
  observer = uni.createIntersectionObserver(instance.proxy)
  observer.relativeToViewport({ top: 300, bottom: 300 }).observe('.private-image-slot', result => {
    if (turn === generation && result.intersectionRatio > 0) void load()
  })
}
onMounted(() => { mounted = true; schedule() })
watch(
  () => [props.school, props.source, props.variant],
  schedule,
  { immediate: true },
)
function preview() {
  if (local.value) uni.previewImage({ urls: [local.value] })
}
</script>
<template>
  <view class="private-image-slot">
  <image v-if="local && !error" class="post-image" :class="{ 'feed-image': variant === 'feed' }" :src="local" :mode="variant === 'feed' ? 'aspectFill' : 'widthFix'" @error="error = true" @tap.stop="preview" />
  <view v-else class="image-placeholder" :class="{ 'feed-placeholder': variant === 'feed' }" :role="error ? 'button' : undefined" :aria-label="error ? '图片加载失败，点击重试' : '图片加载中'" @tap.stop="error && load()">
    {{ error ? '图片加载失败，点击重试' : '图片加载中…' }}
  </view>
  </view>
</template>
<style scoped>
.feed-image, .feed-placeholder {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 152rpx;
  height: 152rpx;
  box-sizing: border-box;
  margin: 0;
  border-radius: 12rpx;
}
.feed-placeholder { padding: 12rpx; font-size: var(--linjian-font-caption); line-height: 1.4; }
</style>
