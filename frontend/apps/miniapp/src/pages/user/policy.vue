<script setup lang="ts">
import { ref } from 'vue'
import { onLoad, onUnload } from '@dcloudio/uni-app'
import { legalDocuments, legalTitles, type LegalDocument, type LegalDocumentKind } from '../../features/legal-documents/api'
const document = ref<LegalDocument>()
const error = ref('')
const busy = ref(false)
const kind = ref<LegalDocumentKind>()
let generation = 0
async function load() {
  if (!kind.value) return
  const turn = ++generation
  busy.value = true; error.value = ''; document.value = undefined
  try { const value = await legalDocuments.read(kind.value); if (turn === generation) document.value = value }
  catch (failure) { if (turn === generation) error.value = failure instanceof Error ? failure.message : '该内容暂未提供' }
  finally { if (turn === generation) busy.value = false }
}
onLoad((query) => {
  if (query?.kind !== 'user_agreement' && query?.kind !== 'privacy_policy') { error.value = '该内容暂未提供'; return }
  kind.value = query.kind
  uni.setNavigationBarTitle({ title: legalTitles[kind.value!] })
  void load()
})
onUnload(() => { generation++ })
function back() { uni.navigateBack() }
</script>
<template>
  <view class="page">
    <text class="heading">{{ kind ? legalTitles[kind] : '协议与政策' }}</text>
    <text v-if="busy" class="muted">正在加载…</text>
    <view v-if="error" class="card"><text class="error">该内容暂未提供或读取失败：{{ error }}</text><button v-if="kind" @tap="load">重试</button></view>
    <view v-if="document" class="card"><text class="muted">版本 {{ document.revision }} · {{ new Date(document.updated_at).toLocaleString() }}</text><text class="body" selectable>{{ document.content }}</text></view>
    <button @tap="back">返回</button>
  </view>
</template>
