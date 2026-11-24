<script setup lang="ts">
import { computed, onUnmounted } from 'vue'
import { credentials } from '../../app/credentials'
import { handlePageError } from '../../shared/use-page'
import { onScopeChange } from './context'
import { location, type Coordinates } from './location'
import { createSchoolRecommendations } from './school-recommendations'
import type { SchoolRecommendation, Cancellation } from './api'
const props = defineProps<{ request: (coordinates: Coordinates, signal: Cancellation) => Promise<SchoolRecommendation[]>; disabled?: boolean }>()
const emit = defineEmits<{ choose: [school: SchoolRecommendation] }>()
const controller = createSchoolRecommendations({ ...location, request: (coordinates, signal) => props.request(coordinates, signal), failure: handlePageError })
const { state, error, items } = controller
const busy = computed(() => ['authorizing', 'locating', 'loading'].includes(state.value))
const stopScope = onScopeChange(controller.clear), stopCredentials = credentials.subscribe(controller.clear)
onUnmounted(() => { controller.clear(); stopScope(); stopCredentials() })
function choose(school: SchoolRecommendation) { controller.clear(); emit('choose', school) }
async function settings() {
  controller.clear()
  try { await uni.openSetting({}); await uni.getSetting({}) }
  catch { error.value = '无法打开授权设置，请在微信设置中检查，或手动搜索学校' }
}
function privacy() { void uni.openPrivacyContract({}) }
defineExpose({ clear: controller.clear })
</script>
<template>
  <view class="card">
    <text class="title">推荐最近的学校</text>
    <text class="muted">仅将本次位置发送至 Linjian，最多推荐 5 所学校以减少搜索。不会自动选中或绑定，你也可以直接搜索其他学校。</text>
    <template v-if="state === 'privacy'">
      <text>请阅读微信隐私保护指引，同意本次位置使用后再继续。</text>
      <button @tap="privacy">查看隐私保护指引</button>
      <button id="linjian-school-location-privacy" :disabled="disabled" open-type="agreePrivacyAuthorization" @agreeprivacyauthorization="controller.recommend()">同意并继续推荐</button>
      <button @tap="controller.decline">不同意，手动选择</button>
    </template>
    <button v-else :disabled="busy || disabled" :loading="busy" @tap="controller.recommend()">{{ busy ? '正在推荐学校' : '推荐附近学校' }}</button>
    <text v-if="error" class="error">{{ error }}</text>
    <button v-if="state === 'denied'" :disabled="disabled" @tap="settings">去设置开启位置权限</button>
    <text v-if="state === 'empty'" class="muted">暂无可推荐学校，请搜索选择。</text>
    <button v-for="school in items" :key="school.id" :disabled="disabled" @tap="choose(school)">
      {{ school.short_name || school.name }} · {{ school.province }} {{ school.city }} · 约 {{ school.distance_m < 1000 ? `${Math.round(school.distance_m)} 米` : `${(school.distance_m / 1000).toFixed(1)} 公里` }}（直线距离）
    </button>
    <text class="muted">不是你的学校？请使用下方搜索选择其他学校。</text>
  </view>
</template>
