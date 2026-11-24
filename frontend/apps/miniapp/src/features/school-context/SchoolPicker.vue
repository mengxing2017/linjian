<script setup lang="ts">
import { mergePageWindow } from '../../shared/page-window'
import PageWindow from '../../shared/PageWindow.vue'
import { shallowRef, ref, computed, onUnmounted, watch } from 'vue'
import { resumeReturnTarget } from '../identity/resume-target'
import { bindSchool, loadSchools, schoolContext, selectSchool } from './context'
import { schoolsApi, type SchoolDisplay, type Cancellation } from './api'
import { session, restoreSession } from '../identity/session'
import { usePage, confirm, navigate } from '../../shared/use-page'
import PageStatus from '../../shared/PageStatus.vue'
import BrandImage from './BrandImage.vue'
import RegionPicker from './RegionPicker.vue'
import SchoolRecommendations from './SchoolRecommendations.vue'
import type { Coordinates } from './location'
import { credentials } from '../../app/credentials'
import { scopeKey } from './context'
import { formatRegion } from '@linjian/china-regions'
const directory = shallowRef<SchoolDisplay[]>([]),
  search = ref(''),
  province = ref(''),
  city = ref(''),
  page = ref(1),
  more = ref(false)
const recommendations = ref<InstanceType<typeof SchoolRecommendations>>()
let visible = true
function hide() { visible = false; recommendations.value?.clear() }
onUnmounted(hide)
watch([search, province, city], () => recommendations.value?.clear())
async function recommend(coordinates: Coordinates, signal: Cancellation) {
  const result = await schoolsApi.recommendations({ ...coordinates, scene: session.user?.bound_school_id ? 'browsing' : 'binding' }, signal)
  return result.items
}
const { busy, error, run } = usePage(() => {
  directory.value = []
  more.value = false
})
const choices = computed(() =>
  session.user?.bound_school_id
    ? schoolContext.schools.filter(
        (s) =>
          (s.name + s.short_name + s.area).includes(search.value) &&
          (!province.value || s.province === province.value) &&
          (!city.value || s.city === city.value),
      )
    : directory.value,
)
function load(reset = true, targetPage?: number) {
  recommendations.value?.clear()
  return run(async (current) => {
    await restoreSession()
    if (!session.user) {
      uni.redirectTo({ url: '/pages/user/login' })
      return
    }
    await loadSchools()
    if (session.user.bound_school_id) return
    const next = targetPage ?? (reset ? 1 : page.value + 1)
    const result = await schoolsApi.directory({
      page: next,
      search: search.value,
      province: province.value,
      city: city.value,
    })
    if (current()) {
      directory.value = mergePageWindow(reset ? [] : directory.value, reset ? 0 : page.value, result.items, next)
      page.value = next
      more.value = !!result.page?.has_more
    }
  })
}
function choose(id: string, name: string) {
  recommendations.value?.clear()
  const revision = credentials.revision, scope = scopeKey()
  void run(async (current) => {
    const valid = () => current() && visible && revision === credentials.revision && scope === scopeKey()
    if (!valid()) return
    if (session.user?.bound_school_id) selectSchool(id)
    else {
      if (!(await confirm(`确认绑定 ${name}？绑定后不能自行换校。`))) return
      if (!valid()) return
      await bindSchool(id)
    }
    if (!visible || revision !== credentials.revision) return
    if (!(await resumeReturnTarget())) uni.reLaunch({ url: '/pages/index/index' })
  })
}
defineExpose({ show: () => { visible = true; return load() }, hide })
</script>
<template>
  <view class="page">
    <text class="heading">{{ session.user?.bound_school_id ? '选择学校' : '绑定学校' }}</text>
    <text class="muted">选择浏览范围不会增加学校权限；绑定后不能自行换校。</text>
    <button v-if="session.user?.bound_school_id" @tap="navigate('/pages/user/schoolChange')">申请更换学校 / 查询结果</button>
    <SchoolRecommendations ref="recommendations" :request="recommend" :disabled="busy" @choose="school => choose(school.id, school.name)" />
    <input v-model="search" class="input" placeholder="搜索名称 / 简称 / 区域" />
    <RegionPicker v-model:province="province" v-model:city="city" />
    <button :disabled="busy" @tap="load()">搜索学校</button>
    <PageStatus :busy="busy" :error="error" :empty="!choices.length" @retry="load()" />
    <text v-if="session.user?.bound_school_id && !schoolContext.schools.length">
      学校暂不可用，请联系管理员
    </text>
    <PageWindow v-if="!session.user?.bound_school_id" :page="page" :busy="busy" @previous="target => load(false, target)" />
    <button
      v-for="school in choices"
      :key="school.id"
      class="card"
      :disabled="busy"
      @tap="choose(school.id, school.name)"
    >
      <BrandImage
        :school="school.id"
        purpose="logo"
        :revision="school.revision"
        :enabled="!!school.logo_url"
      />
      <text>{{ school.short_name || school.name }}</text>
      <text class="muted">
        {{ formatRegion(school) || '地区未填写' }}
      </text>
    </button>
    <button v-if="more && !session.user?.bound_school_id" :disabled="busy" @tap="load(false)">
      {{ page % 3 === 0 ? '下一组学校' : '加载更多学校' }}
    </button>
  </view>
</template>
