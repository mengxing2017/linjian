<script setup lang="ts">
import { ref } from 'vue'
import { onLoad } from '@dcloudio/uni-app'
import { useProtectedScreen } from '../../app/page-lifecycle'
import Screen from '../../features/operations/ServiceInfo.vue'
const id = ref('')
const { screen, allowed } = useProtectedScreen()
onLoad(query => { id.value = query?.id ?? '' })
</script>
<template><Screen v-if="allowed" ref="screen" :id="id" /></template>
