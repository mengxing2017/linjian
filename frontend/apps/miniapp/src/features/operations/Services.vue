<script setup lang="ts">
import { ref, shallowRef, onUnmounted, getCurrentInstance, nextTick } from 'vue'
import { serviceApi, type EffectiveService } from './api'
import { schoolContext, scopeKey, requireSchool, onScopeChange } from '../school-context/context'
import { imagePath, releaseImages } from '../media/files'
import { mapConcurrent } from '../../shared/concurrency'
import { mergePageWindow, windowStart } from '../../shared/page-window'
import PageWindow from '../../shared/PageWindow.vue'
import { uuid } from './policy'
import { sendServiceEvent } from './service-metrics'
const items = shallowRef<EffectiveService[]>([]), categories = ref<{ id: string; name: string }[]>([]), search = ref(''), category = ref(''), error = ref(''), busy = ref(false), more = ref(false), paths = ref<Record<string, string>>({})
const page = ref(1), imageOwner = {}
let generation = 0, active = false, cycle = '', offset = 0
const instance = getCurrentInstance(), observers = new Map<string, ReturnType<typeof uni.createIntersectionObserver>>(), exposed = new Set<string>()
function disconnect() { for (const observer of observers.values()) observer.disconnect(); observers.clear(); exposed.clear() }
function hide() { generation++; active = false; busy.value = false; items.value = []; paths.value = {}; disconnect(); releaseImages(imageOwner) }
const stop = onScopeChange(hide)
onUnmounted(() => { stop(); hide() })
async function load(reset = true, targetPage?: number) {
  if ((busy.value && !reset) || !(await requireSchool())) return
  active = true; busy.value = true; error.value = ''
  const next = targetPage ?? (reset ? 1 : page.value + 1)
  const replace = reset || windowStart(next) !== windowStart(page.value)
  const scope = scopeKey(), turn = ++generation, school = schoolContext.id
  const current = () => active && scope === scopeKey() && turn === generation
  try {
    const value = await serviceApi.catalog(school, { page: next, size: 24, search: search.value, category_id: category.value })
    if (!active || scope !== scopeKey() || turn !== generation) return
    if (replace) { disconnect(); releaseImages(imageOwner); paths.value = {}; cycle = uuid() }
    offset = Date.parse(value.server_time) - Date.now()
    items.value = mergePageWindow(reset ? [] : items.value, reset ? 0 : page.value, value.items, next); categories.value = value.categories; more.value = value.has_more; page.value = next
    for (let start = 0; start < value.items.length && current(); start += 3) {
      const batch = await mapConcurrent(value.items.slice(start, start + 3), async item => {
        try { return [item.id, await imagePath(school, item.image_url, imageOwner)] as const } catch { return undefined }
      }, current)
      if (!current()) return
      paths.value = { ...paths.value, ...Object.fromEntries(batch.filter(item => item !== undefined)) }
    }
  } catch (cause) { if (scope === scopeKey() && turn === generation) error.value = cause instanceof Error ? cause.message : '服务加载失败' }
  finally { if (turn === generation) busy.value = false }
}
async function loaded(item: EffectiveService) {
  const turn = generation
  await nextTick()
  if (!active || !instance || turn !== generation || observers.has(item.id) || exposed.has(item.id)) return
  const observer = uni.createIntersectionObserver(instance.proxy)
  observers.set(item.id, observer)
  observer.relativeToViewport().observe(`#service-${item.id}`, result => {
    if (!active || observers.get(item.id) !== observer || result.intersectionRatio <= 0 || exposed.has(item.id) || !paths.value[item.id]) return
    if (item.ends_at && Date.parse(item.ends_at) <= Date.now() + offset) return
    exposed.add(item.id)
    observer.disconnect()
    observers.delete(item.id)
    void sendServiceEvent(schoolContext.id, { event_id: uuid(), resource_id: item.id, event_type: 'view', display_cycle: cycle, occurred_at: new Date(Date.now() + offset).toISOString(), channel: null, target_version_id: null, placement: 'more_services', attempt_id: null })
  })
}
function open(item: EffectiveService) { uni.navigateTo({ url: `/pages/operations/service?id=${encodeURIComponent(item.id)}` }) }
defineExpose({ show: () => load(), hide, refresh: () => load(), loadMore: () => more.value ? load(false) : undefined })
</script>
<template><view class="page">
  <input v-model="search" placeholder="搜索服务" confirm-type="search" @confirm="load()" /><button size="mini" @tap="load()">搜索</button>
  <scroll-view scroll-x><view class="category-list"><button size="mini" @tap="category = ''; load()">全部</button><button v-for="item in categories" :key="item.id" size="mini" :class="{ selected: category === item.id }" @tap="category = item.id; load()">{{ item.name }}</button></view></scroll-view>
  <view v-if="error" @tap="load()">{{ error }}，点击重试</view><view v-if="!busy && !error && !items.length">暂无可用服务</view>
  <PageWindow :page="page" :busy="busy" @previous="target => load(false, target)" />
  <view v-for="item in items" :id="`service-${item.id}`" :key="item.id" class="service-row" @tap="open(item)">
    <image v-if="paths[item.id]" :src="paths[item.id]" mode="aspectFit" @load="loaded(item)" /><view><text>{{ item.title }} {{ item.badge }}</text><view class="muted">{{ item.subtitle }}</view></view>
  </view><button v-if="more" :loading="busy" @tap="load(false)">{{ page % 3 === 0 ? '下一组' : '加载更多' }}</button>
</view></template>
<style scoped>.category-list{display:flex;gap:12rpx;white-space:nowrap}.service-row{display:flex;gap:24rpx;padding:24rpx}.service-row image{width:80rpx;height:80rpx}</style>
