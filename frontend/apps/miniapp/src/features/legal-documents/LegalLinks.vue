<script setup lang="ts">
import { ref } from 'vue'
import MenuItem from '../../shared/MenuItem.vue'
import { documentIcon } from '../../shared/icons'
import { legalDocuments, legalTitles, type LegalSummary } from './api'
const documents = ref<LegalSummary[]>([])
const error = ref('')
let generation = 0
async function show() {
  const turn = ++generation
  error.value = ''
  try { const result = await legalDocuments.list(); if (turn === generation) documents.value = result }
  catch (failure) { if (turn === generation) error.value = failure instanceof Error ? failure.message : '协议与政策加载失败' }
}
function open(kind: string) { uni.navigateTo({ url: `/pages/user/policy?kind=${kind}` }) }
defineExpose({ show, hide: () => { generation++ } })
</script>
<template>
  <view v-if="documents.length" class="card menu">
    <MenuItem v-for="document in documents" :key="document.kind" :title="legalTitles[document.kind]" :icon="documentIcon(document.kind)" @activate="open(document.kind)" />
  </view>
  <view v-if="error" class="card"><text class="error">协议与政策加载失败：{{ error }}</text><button @tap="show">重试</button></view>
</template>
