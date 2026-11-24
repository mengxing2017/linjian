<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { iconPath } from './icons'
const props = withDefaults(defineProps<{ name: string; selected?: boolean; size?: number }>(), { selected: false, size: 36 })
const source = computed(() => iconPath(props.name, props.selected))
const failed = ref(false)
const dimensions = computed(() => {
  const size = Number.isFinite(props.size) ? Math.min(96, Math.max(16, props.size)) : 36
  return { width: `${size}rpx`, height: `${size}rpx` }
})
watch(source, () => { failed.value = false }, { flush: 'sync' })
function imageFailed(event: unknown) {
  const target = (event as { currentTarget?: { dataset?: { source?: string } } } | null)?.currentTarget
  if (target?.dataset?.source === source.value) failed.value = true
}
</script>
<template>
  <view class="app-icon" :style="dimensions" aria-hidden="true">
    <image v-if="!failed" :key="source" class="app-icon-image" :src="source" :data-source="source" mode="aspectFit" @error="imageFailed" />
    <view v-else class="app-icon-placeholder" />
  </view>
</template>
<style scoped>
.app-icon { display: inline-flex; vertical-align: middle; flex-shrink: 0; align-items: center; justify-content: center; margin-right: 8rpx; }
.app-icon-image { width: 100%; height: 100%; }
.app-icon-placeholder { width: 60%; height: 60%; border: 2rpx solid #9ca3af; border-radius: 6rpx; box-sizing: border-box; }
</style>
