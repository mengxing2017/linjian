<script setup lang="ts">
import { previousWindow, windowStart } from './page-window'
import { watch } from 'vue'
const props = defineProps<{ page: number; busy?: boolean; anchor?: string }>()
const emit = defineEmits<{ previous: [page: number] }>()
watch(() => props.page, (page, previous) => {
  if (windowStart(page) === windowStart(previous)) return
  void uni.pageScrollTo(props.anchor ? { selector: props.anchor, duration: 0 } : { scrollTop: 0, duration: 0 })
})
</script>
<template>
  <view v-if="page > 3" class="page-window row">
    <button size="mini" :disabled="busy" @tap="emit('previous', previousWindow(props.page))">上一组</button>
    <text class="muted">当前展示第 {{ windowStart(page) }}—{{ page }} 页，每组最多三页</text>
  </view>
</template>
