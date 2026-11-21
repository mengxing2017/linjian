<script setup lang="ts">
import { ref, onUnmounted } from 'vue'
import { serviceApi } from './api'
import { requireSchool, schoolContext, scopeKey, onScopeChange } from '../school-context/context'
import { sendServiceOutcome } from './service-metrics'
import { uuid } from './policy'
const props = defineProps<{ id: string; channel: string; version:string;click:string }>()
const url = ref(''), error = ref('')
let generation = 0, reported=false, clockOffset=0
function hide() { generation++; url.value = ''; reported=false }
const stop = onScopeChange(hide)
onUnmounted(() => { stop(); hide() })
async function show() {
  hide(); error.value = ''
  if (!(await requireSchool())) return
  const scope = scopeKey(), turn = generation
  try {
    const value = await serviceApi.resolve(schoolContext.id, props.id,props.version,props.channel)
    if (scope !== scopeKey() || turn !== generation) return
    const action = value.channels.find(channel => channel.key === props.channel)?.action
    if (action?.kind !== 'web_view' || !action.url.startsWith('https://')) throw new Error('服务渠道不可用')
    if(value.action.kind!=='target_channel'||value.action.version_id!==props.version)throw new Error('服务版本已变化，请返回重新选择')
    clockOffset=Date.parse(value.server_time??'')-Date.now();if(!Number.isFinite(clockOffset))clockOffset=0
    url.value = action.url
  } catch (cause) { if (turn === generation) error.value = cause instanceof Error ? cause.message : '网页暂不可用' }
}
function outcome(success:boolean) {
  if(reported||!url.value)return
  reported=true
  const scope=scopeKey()
  sendServiceOutcome(schoolContext.id,scope,Promise.resolve(true),{event_id:uuid(),click_id:props.click,attempt_id:props.click,stage:'webview_load',outcome:success?'webview_loaded':'webview_failed',occurred_at:new Date(Date.now()+clockOffset).toISOString()})
  if(!success){url.value='';error.value='网页加载失败，请返回选择其他服务渠道'}
}
defineExpose({ show, hide, refresh: show })
</script>
<template><web-view v-if="url" :src="url" @load="outcome(true)" @error="outcome(false)" /><view v-else>{{ error || '正在核验服务…' }}</view></template>
