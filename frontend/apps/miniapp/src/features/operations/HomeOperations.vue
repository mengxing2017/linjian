<script setup lang="ts">
import { ref, shallowRef, onUnmounted } from 'vue'
import { operationsApi, type Display, type OperationResource } from './api'
import { uuid, shouldPopup, popupKey } from './policy'
import { imagePath } from '../media/files'
import { executeResourceAction } from '../resources/actions'
import { schoolContext, scopeKey, onScopeChange } from '../school-context/context'
import { session } from '../identity/session'
const data = shallowRef<Display>(), paths = ref<Record<string, string>>({}), index = ref(0), qrVisible = ref(false), qrPath = ref('')
const error = ref('')
let active = false, loading = false, generation = 0, cycle = '', offset = 0, timer: ReturnType<typeof globalThis.setTimeout> | undefined
const loaded = new Set<string>(), exposed = new Set<string>()
function now() { return Date.now() + offset }
function current(v: OperationResource) { return v.active && (!v.starts_at || Date.parse(v.starts_at) <= now()) && (!v.ends_at || Date.parse(v.ends_at) > now()) }
function reset() { generation++; data.value = undefined; paths.value = {}; qrVisible.value = false; qrPath.value = ''; index.value = 0; loaded.clear(); exposed.clear(); if (timer) globalThis.clearTimeout(timer) }
const stop = onScopeChange(() => { reset(); active = false; loading = false })
onUnmounted(() => { stop(); reset(); active = false })
function metric(v: OperationResource, type: 'view' | 'click') {
  if (!active || !current(v)) return
  const event = { event_id: uuid(), resource_id: v.id, event_type: type, display_cycle: cycle, occurred_at: new Date(now()).toISOString() }
  // Metrics are best effort and must never block rendering or navigation.
  void operationsApi.event(schoolContext.id, event).catch(() => {})
}
function expose() { const v = data.value?.banners[index.value]; if (v && loaded.has(v.id) && !exposed.has(v.id) && active) { exposed.add(v.id); metric(v, 'view') } }
function imageLoaded(v: OperationResource) { loaded.add(v.id); expose() }
function changed(e: { detail: { current: number } }) { index.value = e.detail.current; expose() }
function expire() {
  const deadlines = [...(data.value?.banners ?? []), ...(data.value?.qrcode ? [data.value.qrcode] : [])].map(v => v.ends_at ? Date.parse(v.ends_at) : Infinity).filter(v => v > now() && Number.isFinite(v))
  if (deadlines.length) timer = globalThis.setTimeout(() => { if (!active) return; qrVisible.value = false; const v = data.value; if (v) data.value = { ...v, qrcode: v.qrcode && current(v.qrcode) ? v.qrcode : null, banners: v.banners.filter(current) }; index.value = 0; expire() }, Math.min(Math.min(...deadlines) - now() + 10, 2147483647))
}
async function show(force = false) {
  if ((active && !force) || loading || !schoolContext.id || !session.user) return
  active = true; loading = true; reset(); cycle = uuid(); error.value = ''
  const key = scopeKey(), turn = generation, school = schoolContext.id, user = session.user.id
  try {
    const v = await operationsApi.display(school)
    if (key !== scopeKey() || turn !== generation || !active) return
    offset = Date.parse(v.server_time) - Date.now(); data.value = v
    expire()
    await Promise.all(v.banners.map(async r => { if (!r.image_url) return; try { const p = await imagePath(school, r.image_url); if (key === scopeKey() && turn === generation && active) paths.value = { ...paths.value, [r.id]: p } } catch { /* A failed image never creates an impression. */ } }))
    if (key !== scopeKey() || turn !== generation || !active) return
    const qr = v.qrcode
    if (qr?.image_url && current(qr)) {
      const p = await imagePath(school, qr.image_url)
      if (key !== scopeKey() || turn !== generation || !active) return
      qrPath.value = p
      const last = Number(uni.getStorageSync(popupKey(user, school, qr.id))) || 0
      if (shouldPopup(qr, last, now())) qrVisible.value = true
    }
  } catch (e) { if (key === scopeKey() && turn === generation) error.value = e instanceof Error ? e.message : '展示配置加载失败' }
  finally { if (turn === generation) loading = false }
}
function qrLoaded() {
  const qr = data.value?.qrcode
  if (qrVisible.value && qr && current(qr) && session.user) uni.setStorageSync(popupKey(session.user.id, schoolContext.id, qr.id), now())
}
function manual() { if (data.value?.qrcode && current(data.value.qrcode) && qrPath.value) qrVisible.value = true; else uni.showToast({ title: '本校群二维码暂未配置', icon: 'none' }) }
function click(v: OperationResource) { metric(v, 'click'); if (v.action_available === false) { uni.showToast({ title: '目标内容已不可用', icon: 'none' }); return }; void executeResourceAction(v.action).catch(() => uni.showToast({ title: '页面打开失败', icon: 'none' })) }
function previewQr() { if(qrPath.value) uni.previewImage({urls:[qrPath.value]}) }
function hide() { active = false; loading = false; reset() }
defineExpose({ show, hide, refresh: () => show(true) })
</script>
<template>
  <view>
    <swiper v-if="data?.banners.length" :autoplay="active" circular indicator-dots style="height: 260rpx" @change="changed">
      <swiper-item v-for="v in data.banners" :key="v.id"><image v-if="paths[v.id]" :src="paths[v.id]" :alt="v.title" mode="aspectFill" style="width:100%;height:100%" @load="imageLoaded(v)" @tap="click(v)" /></swiper-item>
    </swiper>
    <button size="mini" @tap="manual">查看群二维码</button>
    <view v-if="error" class="muted" @tap="show(true)">展示配置加载失败，点击重试</view>
    <view v-if="qrVisible && data?.qrcode" class="operations-mask" @tap="qrVisible = false"><view class="operations-dialog" @tap.stop>
      <text class="heading">{{ data.qrcode.title }}</text><text>{{ data.qrcode.content }}</text>
      <image :src="qrPath" mode="aspectFit" style="width: 100%; height: 500rpx" @load="qrLoaded" @tap="previewQr" />
      <button @tap="qrVisible = false">关闭</button>
    </view></view>
  </view>
</template>
<style scoped>
.operations-mask{position:fixed;inset:0;background:rgba(0,0,0,.5);z-index:100;display:flex;align-items:center;justify-content:center}.operations-dialog{width:80%;padding:30rpx;background:white;border-radius:20rpx;display:flex;flex-direction:column;gap:20rpx}
</style>
