<script setup lang="ts">
import { shallowRef, ref, computed } from 'vue'
import { resumeReturnTarget } from '../identity/resume-target'
import { bindSchool, loadSchools, schoolContext, selectSchool } from './context'
import { schoolsApi, type SchoolDisplay } from './api'
import { session, restoreSession } from '../identity/session'
import { usePage, confirm } from '../../shared/use-page'
import PageStatus from '../../shared/PageStatus.vue'
import BrandImage from './BrandImage.vue'
const directory = shallowRef<SchoolDisplay[]>([]),
  search = ref(''),
  city = ref(''),
  page = ref(1),
  more = ref(false)
const { busy, error, run } = usePage(() => {
  directory.value = []
  more.value = false
})
const choices = computed(() =>
  session.user?.bound_school_id
    ? schoolContext.schools.filter(
        (s) =>
          (s.name + s.short_name + s.area).includes(search.value) &&
          (!city.value || s.city === city.value),
      )
    : directory.value,
)
function load(reset = true) {
  return run(async (current) => {
    await restoreSession()
    if (!session.user) {
      uni.redirectTo({ url: '/pages/user/login' })
      return
    }
    await loadSchools()
    if (session.user.bound_school_id) return
    const next = reset ? 1 : page.value + 1
    const result = await schoolsApi.directory({
      page: next,
      search: search.value,
      city: city.value,
    })
    if (current()) {
      directory.value = reset ? result.items : [...directory.value, ...result.items]
      page.value = next
      more.value = !!result.page?.has_more
    }
  })
}
function choose(id: string, name: string) {
  void run(async () => {
    if (session.user?.bound_school_id) selectSchool(id)
    else {
      if (!(await confirm(`确认绑定 ${name}？绑定后不能自行换校。`))) return
      await bindSchool(id)
    }
    if (!(await resumeReturnTarget())) uni.reLaunch({ url: '/pages/index/index' })
  })
}
defineExpose({ show: () => load() })
</script>
<template>
  <view class="page">
    <text class="heading">{{ session.user?.bound_school_id ? '选择学校' : '绑定学校' }}</text>
    <text class="muted">选择浏览范围不会增加学校权限；绑定后不能自行换校。</text>
    <input v-model="search" class="input" placeholder="搜索名称 / 简称 / 区域" />
    <input v-model="city" class="input" placeholder="筛选城市" />
    <button :disabled="busy" @tap="load()">搜索学校</button>
    <PageStatus :busy="busy" :error="error" :empty="!choices.length" @retry="load()" />
    <text v-if="session.user?.bound_school_id && !schoolContext.schools.length">
      学校暂不可用，请联系管理员
    </text>
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
        {{ [school.province, school.city, school.area].filter(Boolean).join(' ') }}
      </text>
    </button>
    <button v-if="more && !session.user?.bound_school_id" :disabled="busy" @tap="load(false)">
      加载更多学校
    </button>
  </view>
</template>
