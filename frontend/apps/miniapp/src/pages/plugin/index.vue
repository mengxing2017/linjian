<script setup lang="ts">
import { schoolTheme } from '../../features/school-context/theme'
import { useProtectedScreen } from '../../app/page-lifecycle'
const { screen, allowed } = useProtectedScreen()
import Screen from '../../features/school-context/SchoolInfo.vue'
</script>
<template>
  <view :style="schoolTheme"><Screen v-if="allowed" ref="screen" /></view>
</template>
