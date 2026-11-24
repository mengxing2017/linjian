<script setup lang="ts">
import { schoolTheme } from '../../features/school-context/theme'
import { useProtectedScreen } from '../../app/page-lifecycle'
const { screen, allowed } = useProtectedScreen()
import Screen from '../../features/community/Feed.vue'
</script>
<template>
  <view class="root-page" :style="schoolTheme"><Screen v-if="allowed" ref="screen" home-page /></view>
</template>
