<script setup lang="ts">
import { ref, shallowRef, onUnmounted, nextTick } from 'vue'
import { serviceApi, type EffectiveService, type EffectiveChannel } from './api'
import { schoolContext, scopeKey, requireSchool, onScopeChange } from '../school-context/context'
import { imagePath, releaseImages } from '../media/files'
import { executeResourceAction } from '../resources/actions'
import { executeServiceChannel } from './service-actions'
import { sendServiceEvent, sendServiceOutcome } from './service-metrics'
import { uuid } from './policy'
const props = defineProps<{ id: string }>()
const service = shallowRef<EffectiveService>(), error = ref(''), busy = ref(false), qrPaths = ref<Record<string, string>>({})
let generation = 0, active = false, expires = 0, offset = 0, cycle = ''
const imageOwner = {}
function hide() { generation++; active = false; service.value = undefined; qrPaths.value = {}; busy.value = false; releaseImages(imageOwner) }
const stop = onScopeChange(hide)
onUnmounted(() => { stop(); hide() })
async function show() {
  hide(); active = true; error.value = ''; cycle = uuid()
  if (!(await requireSchool())) return
  const scope = scopeKey(), turn = generation, school = schoolContext.id
  try {
    const value = await serviceApi.resolve(school, props.id)
    if (!active || scope !== scopeKey() || turn !== generation) return
    offset = Date.parse(value.server_time ?? '') - Date.now(); if (!Number.isFinite(offset)) offset = 0
    service.value = value; expires = Math.min(Date.now() + 60000, value.ends_at ? Date.parse(value.ends_at) - offset : Infinity)
    await nextTick()
    if (!active || scope !== scopeKey() || turn !== generation) return
    void sendServiceEvent(school,{event_id:uuid(),resource_id:value.id,event_type:'view',display_cycle:cycle,occurred_at:new Date(Date.now()+offset).toISOString(),channel:null,target_version_id:null,placement:'service_detail',attempt_id:null})
    for (const channel of value.channels) if (channel.action.kind === 'qr_code') {
      try { const path = await imagePath(school, channel.action.image_url, imageOwner); if (!active || scope !== scopeKey() || turn !== generation) return; qrPaths.value = { ...qrPaths.value, [channel.key]: path } } catch { continue }
    }
  } catch (cause) { if (scope === scopeKey() && turn === generation) error.value = cause instanceof Error ? cause.message : '服务暂不可用' }
}
function event(value: EffectiveService, channel: EffectiveChannel | undefined) {
  const event_id = uuid()
  const target = value.action.kind === 'target_channel' ? value.action.version_id : null
  return { event_id, accepted: sendServiceEvent(schoolContext.id, { event_id, resource_id: value.id, event_type: 'click', display_cycle: cycle, occurred_at: new Date(Date.now() + offset).toISOString(), channel: channel?.key ?? null, target_version_id: target, placement:'service_detail',attempt_id:event_id }) }
}
function access(channel?: EffectiveChannel) {
  const value = service.value
  if (!value || busy.value || !active) return
  if (Date.now() >= expires) { uni.showToast({ title: '配置已刷新，请再次点击', icon: 'none' }); void show(); return }
  if (!channel) {
    if (value.action.kind === 'internal_page') { event(value, undefined); void executeResourceAction(value.action).catch(() => uni.showToast({ title: '页面打开失败', icon: 'none' })) }
    return
  }
  const scope = scopeKey(), school = schoolContext.id, click = event(value, channel), turn = generation, clockOffset = offset
  busy.value = true
  executeServiceChannel(channel, value.id, qrPaths.value[channel.key], outcome => {
    if (scope !== scopeKey()) return
    sendServiceOutcome(school, scope, click.accepted, { event_id: uuid(), click_id: click.event_id, attempt_id:click.event_id,stage:channel.action.kind==='copy_link'?'clipboard_api':channel.action.kind==='qr_code'?'qr_display':'navigation_api',outcome, occurred_at: new Date(Date.now() + clockOffset).toISOString() })
    if (turn !== generation) return
    busy.value = false
    if (outcome === 'fail') uni.showToast({ title: '服务打开失败，可重试或选择其他渠道', icon: 'none' })
  },{version_id:value.action.kind==='target_channel'?value.action.version_id:'',click_id:click.event_id})
}
function complaint() { const value=service.value; if(value) uni.navigateTo({url:`/pages/user/feedback?service_id=${encodeURIComponent(value.id)}&service_title=${encodeURIComponent(value.title)}`}) }
function contact() { if(service.value?.disclosure) uni.setClipboardData({data:service.value.disclosure.contact}) }
defineExpose({ show, hide, refresh: show })
</script>
<template><view class="page">
  <view v-if="error" @tap="show">{{ error }}，点击重试</view>
  <view v-if="service" class="service-info"><text class="heading">{{ service.title }}</text><text>{{ service.subtitle }}</text>
    <view v-if="service.disclosure"><view>服务提供方：{{ service.disclosure.provider }}</view><view>用途：{{ service.disclosure.purpose }}</view><view>{{ service.disclosure.disclosure }}</view><view>联系：{{ service.disclosure.contact }}</view><view>投诉：{{ service.disclosure.complaint }}</view></view>
    <view v-if="service.cooperation">合作披露：{{ service.cooperation.disclosure }}</view>
    <button v-if="service.disclosure" size="mini" @tap="contact">复制服务联系方式</button><button size="mini" @tap="complaint">反馈此服务</button>
    <view v-if="service.channels.length">以下按钮将由你主动访问第三方服务；复制成功不代表已打开，展示二维码不代表已扫码。</view>
    <button v-for="channel in service.channels" :key="channel.key" :disabled="busy || (channel.action.kind === 'qr_code' && !qrPaths[channel.key])" @tap="access(channel)">{{ channel.label }}</button>
    <button v-if="service.action.kind === 'internal_page'" @tap="access()">打开站内页面</button>
  </view>
</view></template>
<style scoped>.service-info{display:flex;flex-direction:column;gap:24rpx}</style>
