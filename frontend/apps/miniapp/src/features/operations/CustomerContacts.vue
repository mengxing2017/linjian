<script setup lang="ts">
import { onUnmounted } from 'vue'
import MenuItem from '../../shared/MenuItem.vue'
import { operationsApi } from './api'
import { imagePath, releaseImages } from '../media/files'
import { scopeKey, onScopeChange } from '../school-context/context'
import { handlePageError, navigate } from '../../shared/use-page'
import { contactEntries, createCustomerContacts } from './use-customer-contacts'
const emit = defineEmits<{ 'retry-prepare': [] }>()
const imageOwner = {}
const contacts = createCustomerContacts({
  display: operationsApi.display,
  image: (school, source) => imagePath(school, source, imageOwner),
  key: scopeKey,
  failure: handlePageError,
  notify: title => uni.showToast({ title, icon: 'none' }),
  preview: path => new Promise<void>((resolve, reject) => uni.previewImage({ urls: [path], success: () => resolve(), fail: reject })),
})
const { state, config, error, failureStage, selected, imageState, path, imageError, title } = contacts
const stop = onScopeChange(() => { contacts.reset(); releaseImages(imageOwner) })
onUnmounted(() => { stop(); contacts.dispose(); releaseImages(imageOwner) })
function retry() {
  if (failureStage.value === 'prepare') emit('retry-prepare')
  else void contacts.retry()
}
defineExpose({ prepare: contacts.prepare, show: contacts.show, hide: () => { contacts.hide(); releaseImages(imageOwner) } })
</script>
<template>
  <view v-if="state !== 'idle' && state !== 'disabled'" class="card menu">
    <text class="heading">平台服务</text>
    <text v-if="state === 'preparing' || state === 'loading'" class="muted">平台服务加载中…</text>
    <view v-else-if="state === 'login-required'">
      <text>登录后查看平台联系方式</text>
      <button @tap="navigate('/pages/user/login')">前往登录</button>
    </view>
    <view v-else-if="state === 'school-required'">
      <text>请先绑定或选择可用学校，再查看平台联系方式</text>
      <button @tap="navigate('/pages/plugin/index')">选择学校</button>
    </view>
    <view v-else-if="state === 'error'">
      <text class="error">平台服务加载失败：{{ error }}</text>
      <button @tap="retry">重试平台服务</button>
    </view>
    <view v-else-if="state === 'ready'">
      <MenuItem v-for="entry in contactEntries" :key="entry.kind" :title="entry.title" :icon="entry.kind" @activate="contacts.open(entry.kind)" />
      <view v-if="selected">
        <text class="heading">{{ title }}</text>
        <text v-if="imageState === 'loading'" class="muted">二维码加载中…</text>
        <image v-if="imageState === 'ready'" :src="path" mode="aspectFit" style="width:100%;height:450rpx" @tap="contacts.preview" />
        <view v-if="imageState === 'error'"><text class="error">{{ imageError }}</text><button @tap="contacts.retryImage">重试二维码</button></view>
        <text>{{ config?.note }}</text>
        <button @tap="contacts.close">关闭</button>
      </view>
    </view>
  </view>
</template>
