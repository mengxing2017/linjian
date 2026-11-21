<script setup lang="ts">
import { ref, shallowRef, onUnmounted, getCurrentInstance, nextTick } from 'vue'
import { serviceApi, type EffectiveService } from './api'
import { schoolContext, scopeKey, onScopeChange } from '../school-context/context'
import { session } from '../identity/session'
import { imagePath } from '../media/files'
import { uuid } from './policy'
import { sendServiceEvent } from './service-metrics'
import OfficialMarketing from './OfficialMarketing.vue'
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
    offset = Date.parse(data.server_time) - Date.now(); items.value = data.featured; hasServices.value = data.items.length > 0
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
    <view class="services-grid"><view v-for="item in items" :id="`service-${item.id}`" :key="item.id" class="service-card" @tap="open(item)">
      <image v-if="paths[item.id]" :src="paths[item.id]" mode="aspectFit" @load="loaded(item)" />
      <text>{{ item.title }}</text><text v-if="item.badge" class="badge">{{ item.badge }}</text><text class="muted">{{ item.subtitle }}</text>
    </view></view>
    <button size="mini" @tap="more">更多服务</button>
  </view><OfficialMarketing ref="marketing" @theme="warm = true" /></view>
</template>
<style scoped>
.services-home{padding:20rpx}.services-grid{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:20rpx}.service-card{display:flex;align-items:center;flex-direction:column;position:relative;font-size:24rpx;gap:8rpx}.service-card image{width:80rpx;height:80rpx}.muted{font-size:20rpx;max-width:100%;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.badge{color:#c34e25;font-size:20rpx}
.warm .services-home{background:#fff4e8;border-radius:24rpx;color:#71430c}
</style>
