<script setup lang="ts">
import { schoolTheme } from '../../features/school-context/theme'
import { useProtectedScreen } from '../../app/page-lifecycle'
import { ref } from 'vue'
import { onLoad } from '@dcloudio/uni-app'
const serviceId=ref(''),serviceTitle=ref('')
onLoad(query=>{serviceId.value=query?.service_id??'';serviceTitle.value=query?.service_title??''})
const { screen, allowed } = useProtectedScreen()
import Screen from '../../features/feedback/Feedback.vue'
</script>
<template>
  <view :style="schoolTheme"><Screen v-if="allowed" ref="screen" :service-id="serviceId" :service-title="serviceTitle" /></view>
</template>
