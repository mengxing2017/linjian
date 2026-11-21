<script setup lang="ts">
import { ref } from 'vue'
import { onLoad } from '@dcloudio/uni-app'
import { useScreen } from '../../app/page-lifecycle'
import Screen from '../../features/operations/ServiceInfo.vue'
const id = ref(''), screen = useScreen()
onLoad(query => { id.value = query?.id ?? '' })
</script>
<template><Screen ref="screen" :id="id" /></template>
