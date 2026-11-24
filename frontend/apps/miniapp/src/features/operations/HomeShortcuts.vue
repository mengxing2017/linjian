<script setup lang="ts">
import { ref, shallowRef, onUnmounted, getCurrentInstance, nextTick } from 'vue'
import { serviceApi, type EffectiveService } from './api'
import { schoolContext, scopeKey, onScopeChange } from '../school-context/context'
import { session } from '../identity/session'
import { imagePath } from '../media/files'
import { uuid } from './policy'
import { sendServiceEvent } from './service-metrics'
import OfficialMarketing from './OfficialMarketing.vue'
import AppIcon from '../../shared/AppIcon.vue'
const marketing = shallowRef<InstanceType<typeof OfficialMarketing>>(), warm = ref(false)
const items = shallowRef<EffectiveService[]>([]), paths = ref<Record<string, string>>({})
const hasServices = ref(false)
const instance = getCurrentInstance()
let active = false, generation = 0, cycle = '', offset = 0
const observers = new Map<string, ReturnType<typeof uni.createIntersectionObserver>>()
const impressions = new Set<string>()
function valid(item: EffectiveService) { return !item.ends_at || Date.parse(item.ends_at) > Date.now() + offset }
function hide() { active = false; generation++; items.value = []; hasServices.value = false; paths.value = {}; for (const observer of observers.values()) observer.disconnect(); observers.clear(); impressions.clear(); marketing.value?.hide(); warm.value = false }
const stop = onScopeChange(hide)
onUnmounted(() => { stop(); hide() })
async function show() {
  hide(); active = true; cycle = uuid()
  void marketing.value?.show()
  if (!schoolContext.id || !session.user) return
  const scope = scopeKey(), turn = generation, school = schoolContext.id
  try {
    const data = await serviceApi.catalog(school)
    if (!active || scope !== scopeKey() || turn !== generation) return
    offset = Date.parse(data.server_time) - Date.now(); items.value = data.featured.slice(0, 3); hasServices.value = data.items.length > 0
    for (const item of items.value) {
      try {
        const path = await imagePath(school, item.image_url)
        if (!active || scope !== scopeKey() || turn !== generation) return
        paths.value = { ...paths.value, [item.id]: path }
      } catch { continue }
    }
  } catch { if (turn === generation) items.value = [] }
}
async function loaded(item: EffectiveService) {
  const turn = generation
  await nextTick()
  if (!active || !instance || turn !== generation || observers.has(item.id)) return
  const observer = uni.createIntersectionObserver(instance.proxy)
  observers.set(item.id, observer)
  observer.relativeToViewport().observe(`#service-${item.id}`, result => {
    if (!active || turn !== generation || result.intersectionRatio <= 0 || !valid(item) || !paths.value[item.id] || impressions.has(item.id)) return
    impressions.add(item.id)
    void sendServiceEvent(schoolContext.id, { event_id: uuid(), resource_id: item.id, event_type: 'view', display_cycle: cycle, occurred_at: new Date(Date.now() + offset).toISOString(), channel: null, target_version_id: null, placement: 'home_shortcut', attempt_id: null })
  })
}
function open(item: EffectiveService) {
  if (!valid(item)) { void show(); return }
  uni.navigateTo({ url: `/pages/operations/service?id=${encodeURIComponent(item.id)}` })
}
function more() { uni.navigateTo({ url: '/pages/operations/services' }) }
defineExpose({ show, hide, refresh: show })
</script>
<template>
  <view :class="{ warm }"><view v-if="hasServices" class="services-home">
    <view class="services-grid"><button v-for="item in items" :id="`service-${item.id}`" :key="item.id" class="service-card" @tap="open(item)">
      <image v-if="paths[item.id]" :src="paths[item.id]" mode="aspectFit" @load="loaded(item)" />
      <view v-else class="service-icon"><AppIcon name="services" :size="40" /></view>
      <text class="service-title">{{ item.title }}</text><text v-if="item.badge" class="badge">{{ item.badge }}</text>
    </button><button class="service-card" @tap="more"><view class="service-icon"><AppIcon name="services" :size="40" /></view><text class="service-title">更多服务</text></button></view>
  </view><OfficialMarketing ref="marketing" @theme="warm = true" /></view>
</template>
<style scoped>
.services-home{padding:8rpx 0 12rpx}.services-grid{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:8rpx}.service-card{display:flex;align-items:center;flex-direction:column;position:relative;font-size:24rpx;gap:4rpx;min-height:44px;width:100%;min-width:0;margin:0;padding:8rpx 0;line-height:1.4;background:transparent;border-radius:12rpx}.service-card::after{border:0}.service-card image,.service-icon{width:56rpx;height:56rpx;flex-shrink:0}.service-icon{display:flex;align-items:center;justify-content:center;background:#eef3ff;border-radius:16rpx}.service-title{max-width:100%;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.badge{position:absolute;right:0;top:0;max-width:100%;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;color:#9a3412;background:#fff4e8;border-radius:8rpx;font-size:20rpx}
.warm .services-home{background:#fff4e8;border-radius:24rpx;color:#71430c}
</style>
