<script setup lang="ts">
import { ref, watch, onUnmounted } from 'vue'
import { brandingFile, releaseBranding } from './branding'
import { onScopeChange } from './context'
const props = defineProps<{
  school: string
  purpose: 'logo' | 'header'
  revision: number
  enabled: boolean
  compact?: boolean
}>()
const path = ref('')
const owner = {}
let generation = 0
const stop = onScopeChange(() => {
  generation++
  releaseBranding(owner)
  path.value = ''
})
onUnmounted(() => {
  generation++
  releaseBranding(owner)
  stop()
})
watch(
  () => [props.school, props.purpose, props.revision, props.enabled],
  async () => {
    const turn = ++generation
    releaseBranding(owner)
    path.value = ''
    if (!props.enabled) return
    try {
      const value = await brandingFile(props.school, props.purpose, props.revision, owner)
      if (turn === generation) path.value = value
    } catch {
      /* Default branding remains visible. */
    }
  },
  { immediate: true },
)
</script>
<template>
  <image
    v-if="path"
    :src="path"
    mode="aspectFill"
    :class="purpose === 'header' ? 'school-brand-background' : 'school-brand-logo'"
    :style="purpose === 'logo' && compact ? { width: '64rpx', height: '64rpx' } : undefined"
  />
  <view v-else-if="purpose === 'logo'" class="school-brand-logo" :style="compact ? { width: '64rpx', height: '64rpx' } : undefined">校</view>
</template>
<style scoped>
.school-brand-background {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  opacity: 0.45;
  pointer-events: none;
}
.school-brand-logo {
  width: 96rpx;
  height: 96rpx;
  border-radius: 16rpx;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(255, 255, 255, 0.2);
}
</style>
