<script setup lang="ts">
import { schoolTheme } from '../../features/school-context/theme'
import { useProtectedScreen } from '../../app/page-lifecycle'
const { screen, allowed } = useProtectedScreen(true)
import Screen from '../../features/community/Detail.vue'
import { ref } from 'vue'
import { onLoad } from '@dcloudio/uni-app'
const id = ref(''),
  school = ref('')
onLoad((query) => {
  id.value = String(query?.id ?? '')
  school.value = String(query?.school ?? '')
})
</script>
<template>
  <view :style="schoolTheme"><Screen v-if="allowed" ref="screen" :id="id" :school="school" /></view>
</template>
