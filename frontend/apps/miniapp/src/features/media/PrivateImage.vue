<script setup lang="ts">
import { ref, watch, onUnmounted } from 'vue'
import { imagePath } from './files'
import { onScopeChange, clearScope } from '../school-context/context'
const props = withDefaults(defineProps<{ school: string; source: string; variant?: 'detail' | 'feed' }>(), { variant: 'detail' })
const local = ref(''),
  error = ref(false)
let generation = 0
const stop = onScopeChange(() => {
  generation++
  local.value = ''
  error.value = false
})
onUnmounted(() => {
  generation++
  stop()
})
async function load() {
  const turn = ++generation
  local.value = ''
  error.value = false
  try {
    const path = await imagePath(props.school, props.source)
    if (turn === generation) local.value = path
  } catch (failure) {
    if (turn === generation) {
      if (failure && typeof failure === 'object' && 'status' in failure && failure.status === 403)
        clearScope()
      error.value = true
    }
  }
}
watch(
  () => [props.school, props.source],
  () => void load(),
  { immediate: true },
)
function preview() {
  if (local.value) uni.previewImage({ urls: [local.value] })
}
</script>
<template>
  <image v-if="local && !error" class="post-image" :class="{ 'feed-image': variant === 'feed' }" :src="local" :mode="variant === 'feed' ? 'aspectFill' : 'widthFix'" @error="error = true" @tap.stop="preview" />
  <view v-else class="image-placeholder" :class="{ 'feed-placeholder': variant === 'feed' }" :role="error ? 'button' : undefined" :aria-label="error ? '图片加载失败，点击重试' : '图片加载中'" @tap.stop="error && load()">
    {{ error ? '图片加载失败，点击重试' : '图片加载中…' }}
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
