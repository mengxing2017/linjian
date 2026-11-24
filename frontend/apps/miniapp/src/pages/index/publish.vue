<script setup lang="ts">
import { schoolTheme } from '../../features/school-context/theme'
import { useProtectedScreen } from '../../app/page-lifecycle'
const { screen, allowed } = useProtectedScreen()
import Screen from '../../features/community/Publish.vue'
import { ref } from 'vue'
import { onLoad } from '@dcloudio/uni-app'
import { readReturnTarget, savePublishTarget } from '../../features/identity/return-target'
import { session } from '../../features/identity/session'
const id = ref(''),
  school = ref('')
onLoad((query) => {
  id.value = String(query?.id ?? '')
  school.value = String(query?.school ?? '')
  if (!id.value && readReturnTarget()?.kind !== 'publish') savePublishTarget('home', session.user?.id)
})
</script>
<template>
  <view :style="schoolTheme">
    <Screen v-if="allowed" ref="screen" :id="id || undefined" :school="school || undefined" />
  </view>
</template>
