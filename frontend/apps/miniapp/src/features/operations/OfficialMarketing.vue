<script setup lang="ts">
import { ref, onUnmounted } from 'vue'
import { serviceApi, type MarketingSlot } from './api'
import { schoolContext, scopeKey, onScopeChange } from '../school-context/context'
import { session } from '../identity/session'
import { uuid } from './policy'
import { rewardLifecycle } from './marketing-lifecycle'
const emit = defineEmits<{ theme: [] }>()
const slots = ref<MarketingSlot[]>([]), busy = ref(''), failed = ref<string[]>([])
let generation = 0, active = false
const themeGranted = ref(false)
const contexts = new Map<string, ReturnType<typeof rewardLifecycle>>()
function hide() {
  active = false; generation++; slots.value = []; busy.value = ''; failed.value = []; themeGranted.value = false
  for (const context of contexts.values()) context.dispose()
  contexts.clear()
}
const stop = onScopeChange(hide)
onUnmounted(() => { stop(); hide() })
function metric(slot: MarketingSlot, event_type: 'loaded' | 'error' | 'show_api_success' | 'closed_early' | 'client_completed') {
  if (!active) return
  void serviceApi.marketingEvent(schoolContext.id, { event_id: uuid(), slot_id: slot.id, event_type, occurred_at: new Date().toISOString() }).catch(() => {})
}
async function show() {
  hide(); active = true
  if (!schoolContext.id || !session.user) return
  const scope = scopeKey(), turn = generation
  try {
    const value = await serviceApi.marketing(schoolContext.id)
    if (scope === scopeKey() && turn === generation && active) slots.value = value.items
  } catch { if (turn === generation) slots.value = [] }
}
function reward(slot: MarketingSlot) {
  if (!active || busy.value || themeGranted.value || slot.reward_scene !== 'service_theme') return
  const turn = generation, scope = scopeKey()
  let context = contexts.get(slot.id)
  if (!context) {
    if (typeof uni.createRewardedVideoAd !== 'function') { failed.value.push(slot.id); return }
    let ad: ReturnType<typeof uni.createRewardedVideoAd>
    try { ad = uni.createRewardedVideoAd({ adUnitId: slot.ad_unit_id }) }
    catch { metric(slot, 'error'); failed.value.push(slot.id); return }
    context = rewardLifecycle(ad, {
      current: () => active && turn === generation && scope === scopeKey(),
      record: stage => metric(slot, stage),
      completed: () => { if (!themeGranted.value) { themeGranted.value = true; emit('theme') } },
      failed: () => { busy.value = ''; uni.showToast({ title: '营销暂不可用，不影响其他功能；可稍后主动重试', icon: 'none' }) },
      closed: () => { busy.value = '' },
    })
    contexts.set(slot.id, context)
  }
  busy.value = slot.id
  void context.show()
}
defineExpose({ show, hide, refresh: show })
</script>
<template><view v-if="slots.length" class="official-marketing">
  <view v-for="slot in slots" :key="slot.id"><template v-if="!failed.includes(slot.id)">
    <ad-custom v-if="slot.format === 'custom'" :unit-id="slot.ad_unit_id" @load="metric(slot, 'loaded')" @error="failed.push(slot.id); metric(slot, 'error')" />
    <view v-else><text>自愿观看官方激励视频，可切换本次服务区配色；不影响论坛使用，不产生现金、积分或可交易权益。</text><button :disabled="!!busy || themeGranted" @tap="reward(slot)">{{ themeGranted ? '本次配色已解锁' : '自愿观看并切换配色' }}</button></view>
  </template></view>
</view></template>
<style scoped>.official-marketing{padding:20rpx;font-size:24rpx}</style>
