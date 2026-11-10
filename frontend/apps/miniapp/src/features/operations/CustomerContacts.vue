<script setup lang="ts">
import { shallowRef, ref, onUnmounted } from 'vue'
import { operationsApi, type Display } from './api'
import { imagePath } from '../media/files'
import { schoolContext, scopeKey, onScopeChange } from '../school-context/context'
const config = shallowRef<Display['customer']>(), selected = ref(''), path = ref('')
let generation = 0
const clear = () => { generation++; config.value = undefined; selected.value = ''; path.value = '' }
const stop = onScopeChange(clear)
onUnmounted(stop)
async function show() { clear(); if (!schoolContext.id) return; const key = scopeKey(), turn = generation; try { const v = await operationsApi.display(schoolContext.id); if (key === scopeKey() && turn === generation) config.value = v.customer } catch { /* Contact configuration never blocks the profile. */ } }
async function open(kind: 'business' | 'join' | 'service', title: string) { const src = config.value?.[kind]; if (!src) { uni.showToast({ title: '暂未配置联系方式', icon: 'none' }); return }; const key = scopeKey(), turn = generation; try { const p = await imagePath(schoolContext.id, src); if (key === scopeKey() && turn === generation) { path.value = p; selected.value = title } } catch { uni.showToast({ title: '二维码加载失败，请重试', icon: 'none' }) } }
function preview() { if(path.value) uni.previewImage({urls:[path.value]}) }
defineExpose({ show, hide: () => { selected.value = ''; path.value = '' } })
</script>
<template>
  <view v-if="config?.enabled" class="card">
    <button @tap="open('business','商务合作')">商务合作</button><button @tap="open('join','申请加盟')">申请加盟</button><button @tap="open('service','平台客服')">平台客服</button>
    <view v-if="selected"><text class="heading">{{ selected }}</text><image :src="path" mode="aspectFit" style="width:100%;height:450rpx" @tap="preview" /><text>{{ config.note }}</text><button @tap="selected=''">关闭</button></view>
  </view>
</template>
