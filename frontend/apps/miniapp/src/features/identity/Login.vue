<script setup lang="ts">
import { onUnmounted, ref, watch } from 'vue'
import { resumeReturnTarget } from './resume-target'
import { clearReturnTarget } from './return-target'
import { startLogin, acceptLogin } from './session'
import { identityApi } from './api'
import { loadSchools } from '../school-context/context'
import { legalDocuments, legalTitles } from '../legal-documents/api'
import { createOnboarding } from './use-onboarding'
import SchoolRecommendations from '../school-context/SchoolRecommendations.vue'
import { schoolsApi, type SchoolRecommendation, type Cancellation } from '../school-context/api'
import type { Coordinates } from '../school-context/location'
function requestKey() {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, character => {
    const value = Math.floor(Math.random() * 16)
    return (character === 'x' ? value : (value & 3) | 8).toString(16)
  })
}
const form = createOnboarding({
  identity: identityApi, documents: legalDocuments, start: startLogin, requestKey,
  ready: async value => {
    acceptLogin(value)
    if (!(await loadSchools())) throw new Error('学校暂不可用，请重试')
    if (!(await resumeReturnTarget())) await uni.reLaunch({ url: '/pages/index/index' })
  },
})
const { documents, schools, selected, bound, query, more, checked, progress, busy, error, loaded } = form
const recommendations = ref<InstanceType<typeof SchoolRecommendations>>()
const recommendedName = ref('')
let visible = true, lifetime = 0
function hide() { visible = false; lifetime++; recommendations.value?.clear() }
watch(query, () => recommendations.value?.clear())
async function recommend(coordinates: Coordinates, signal: Cancellation) {
  const turn = lifetime
  const ticket = await form.recommendationTicket()
  if (!visible || turn !== lifetime || signal.aborted) throw new Error('推荐已取消')
  try { return (await schoolsApi.onboardingRecommendations({ ...coordinates, ticket }, signal)).items }
  catch (failure) {
    if (visible && turn === lifetime && !signal.aborted && failure && typeof failure === 'object' && 'status' in failure && failure.status === 401) form.dispose()
    throw failure
  }
}
function chooseRecommendation(school: SchoolRecommendation) { recommendedName.value = school.name; form.choose(school.id) }
function choose(id: string) { recommendations.value?.clear(); recommendedName.value = ''; form.choose(id) }
function find(reset = true) { recommendations.value?.clear(); return form.find(reset) }
function submit() { recommendations.value?.clear(); return form.submit() }
function open(kind: string) { uni.navigateTo({ url: `/pages/user/policy?kind=${kind}` }) }
function toggle(event: { detail: { value: string[] } }) { form.toggle(event.detail.value) }
async function cancel() { recommendations.value?.clear(); recommendedName.value = ''; clearReturnTarget(); await form.cancel() }
defineExpose({ show: () => { visible = true; return form.show() }, refresh: form.show, hide })
onUnmounted(() => { hide(); form.dispose() })
</script>
<template>
  <view class="page login">
    <text class="brand">Linjian</text>
    <text class="heading">欢迎加入校园社区</text>
    <text class="muted">选择你的学校，开启校园生活。使用微信身份登录并确认后才会实际绑定学校。</text>
    <view class="card">
      <text class="title">我的学校</text>
      <text v-if="bound" class="muted">将进入账号已绑定的学校，不能在首次入驻入口换校。</text>
      <template v-else>
        <text class="muted">首次推荐会验证微信身份以获取临时入驻凭据，不代表同意项目协议、完成登录或绑定学校。</text>
        <SchoolRecommendations ref="recommendations" :request="recommend" :disabled="busy" @choose="chooseRecommendation" />
        <input v-model="query" class="input" :disabled="busy" placeholder="搜索学校名称、简称或地区" />
        <button :disabled="busy" @tap="find()">搜索学校</button>
        <button v-for="school in schools" :key="school.id" :disabled="busy" :class="{ selected: selected === school.id }" @tap="choose(school.id)">{{ selected === school.id ? '✓ ' : '' }}{{ school.short_name || school.name }} · {{ school.province }} {{ school.city }}</button>
        <text v-if="loaded && !schools.length" class="muted">暂无可选学校，请尝试其他搜索条件。</text>
        <button v-if="more" :disabled="busy" @tap="find(false)">更多学校</button>
      </template>
      <text v-if="selected" class="muted">已选择：{{ schools.find(row => row.id === selected)?.name || recommendedName || '账号绑定学校' }}</text>
    </view>
    <view v-if="documents.length" class="card">
      <text class="title">协议与政策</text>
      <checkbox-group @change="toggle">
        <view v-for="document in documents" :key="`${document.kind}:${document.revision}`" class="row">
          <label><checkbox :value="document.kind" :checked="checked[document.kind] === document.revision" :disabled="busy" />我已阅读并同意</label>
          <text class="selected" @tap.stop="open(document.kind)">《{{ legalTitles[document.kind] }}》</text>
        </view>
      </checkbox-group>
      <text v-if="progress?.first_login === false" class="muted">你已完成首次登录，本次不要求重复确认。</text>
    </view>
    <text v-if="error" class="error">{{ error }}</text>
    <button v-if="error" :disabled="busy" @tap="form.show">重新读取并核对状态</button>
    <button class="primary" :loading="busy" :disabled="busy || !loaded" @tap="submit">微信登录并进入</button>
    <button :disabled="busy" @tap="cancel">暂不登录</button>
    <text class="muted">未完成协议确认与学校准入时不能进入功能页面。暂不登录会留在本页，仍可阅读协议。</text>
  </view>
</template>
