<script setup lang="ts">
import { requireSchool, schoolContext } from './context'
import { usePage, navigate } from '../../shared/use-page'
import PageStatus from '../../shared/PageStatus.vue'
import SchoolHeader from './SchoolHeader.vue'
import { formatRegion } from '@linjian/china-regions'
const { busy, error, run } = usePage()
function load() { return run(async () => { await requireSchool() }) }
defineExpose({ show: load, refresh: load })
</script>
<template>
  <view class="page">
    <text class="heading">我的学校</text>
    <PageStatus :busy="busy" :error="error" @retry="load" />
    <view v-if="schoolContext.school">
      <SchoolHeader :school="schoolContext.school" />
      <view class="card">
        <text class="title">{{ schoolContext.school.name }}</text>
        <text class="muted">{{ formatRegion(schoolContext.school) || '地区未填写' }}</text>
        <text>一个账号仅绑定一所学校，不能直接切换学校。换校须提交申请，审核通过后更新绑定。</text>
      </view>
    </view>
    <button @tap="navigate('/pages/user/schoolChange')">申请换校 / 查看审核结果</button>
  </view>
</template>
