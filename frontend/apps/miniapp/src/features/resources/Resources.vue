<script setup lang="ts">
import { computed, ref, shallowRef } from 'vue'
import type { HomeMenuTag } from '../community/api'
import { resourcesApi, homeMenuApi, type SchoolResource } from './api'
import {executeResourceAction,actionLabel} from './actions'
import { requireSchool, schoolContext, scopeKey } from '../school-context/context'
import { usePage } from '../../shared/use-page'
import PageStatus from '../../shared/PageStatus.vue'
import PrivateImage from '../media/PrivateImage.vue'
const props = defineProps<{ kind?: 'legal' | 'tag' }>()
const selectedKind = ref<'service' | 'legal'>(props.kind === 'legal' ? 'legal' : 'service')
const rows = shallowRef<SchoolResource[]>([])
const tags = shallowRef<HomeMenuTag[]>([])
const visible = computed(() =>
  rows.value.filter((r) =>
    r.kind === selectedKind.value,
  ),
)
const { busy, error, run } = usePage(() => {
  rows.value = []
  tags.value = []
})
function load() {
  return run(async () => {
    if (!(await requireSchool())) return
    const key = scopeKey()
    if (props.kind === 'tag') {
      const result = await homeMenuApi.public(schoolContext.id)
      if (key === scopeKey()) tags.value = result.tags
    } else {
      const result = await resourcesApi.list(schoolContext.id)
      if (key === scopeKey()) rows.value = result
    }
  })
}
function activate(item: SchoolResource) {
 const school=schoolContext.id
 void run(async()=>{await executeResourceAction(item.action);if(item.kind==='banner'&&item.action.kind!=='none')await resourcesApi.click(school,item.id)})
}
defineExpose({ show: load })
</script>
<template>
  <view class="page">
    <text class="heading">
      {{ props.kind === 'tag' ? '校园标签' : '校园信息' }}
    </text>
    <view v-if="props.kind !== 'tag'" class="row tabs">
      <button size="mini" :class="{ selected: selectedKind === 'service' }" @tap="selectedKind = 'service'">校园服务</button>
      <button size="mini" :class="{ selected: selectedKind === 'legal' }" @tap="selectedKind = 'legal'">学校说明</button>
    </view>
    <view v-for="tag in tags" :key="tag.id" class="card"><text class="title">{{ tag.name }}</text></view>
    <view v-for="item in visible" :key="item.id" class="card">
      <text class="title">{{ item.title }}</text>
      <text class="body">{{ item.content }}</text>
      <PrivateImage
        v-if="item.image_url"
        :school="schoolContext.id"
        :source="item.image_url"
      />
      <button v-if="actionLabel(item.action)" size="mini" :disabled="busy" @tap="activate(item)">{{actionLabel(item.action)}}</button>
    </view>
    <PageStatus :busy="busy" :error="error" :empty="props.kind === 'tag' ? !tags.length : !visible.length" @retry="load" />
  </view>
</template>
