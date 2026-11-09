<script setup lang="ts">
import { ref, watch, onUnmounted } from 'vue'
import { avatarFile } from './avatar-files'
import { onScopeChange } from '../school-context/context'
const props = defineProps<{ source?: string | null }>()
const local = ref('')
let generation = 0
const stop = onScopeChange(() => {
  generation++
  local.value = ''
})
onUnmounted(() => {
  generation++
  stop()
})
watch(
  () => props.source,
  async (url) => {
    const turn = ++generation
    local.value = ''
    if (url)
      try {
        const path = await avatarFile(url)
        if (turn === generation) local.value = path
      } catch {
        /* Default avatar also covers a revoked private asset. */
      }
  },
  { immediate: true },
)
</script>
<template>
  <image
    :src="local || '/static/images/avatar.png'"
    style="width: 120rpx; height: 120rpx; border-radius: 50%"
  />
</template>
