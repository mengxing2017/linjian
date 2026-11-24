<script setup lang="ts">
import { ref, watch, onUnmounted } from 'vue'
import { avatarFile, releaseAvatars } from './avatar-files'
import { onScopeChange } from '../school-context/context'
const props = defineProps<{ source?: string | null }>()
const local = ref('')
const owner = {}
let generation = 0
const stop = onScopeChange(() => {
  generation++
  releaseAvatars(owner)
  local.value = ''
})
onUnmounted(() => {
  generation++
  releaseAvatars(owner)
  stop()
})
watch(
  () => props.source,
  async (url) => {
    const turn = ++generation
    releaseAvatars(owner)
    local.value = ''
    if (url)
      try {
        const path = await avatarFile(url, owner)
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
