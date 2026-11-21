<script setup lang="ts">
import { ref } from 'vue'
import { onLoad } from '@dcloudio/uni-app'
import { useScreen } from '../../app/page-lifecycle'
import Screen from '../../features/operations/ServiceWebview.vue'
const id = ref(''), channel = ref(''), version=ref(''),click=ref(''), screen = useScreen()
onLoad(query => { id.value = query?.id ?? ''; channel.value = query?.channel ?? '';version.value=query?.version??'';click.value=query?.click??'' })
</script>
<template><Screen ref="screen" :id="id" :channel="channel" :version="version" :click="click" /></template>
