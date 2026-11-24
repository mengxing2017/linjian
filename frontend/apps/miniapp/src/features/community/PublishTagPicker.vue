<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import type { HomeMenuTag } from './api'
import { emptyPublishTags, publishTagNames, type PublishTags } from './publish-tags'
const props = defineProps<{ open: boolean; candidates: HomeMenuTag[]; selection: PublishTags; disabled?: boolean }>()
const emit = defineEmits<{ close: []; confirm: [value: PublishTags] }>()
const temporary = ref(emptyPublishTags())
const search = ref('')
const notice = ref('')
const keyboardHeight = ref(0)
watch(() => props.open, open => {
  search.value = ''
  notice.value = ''
  keyboardHeight.value = 0
  temporary.value = open ? {
    selected: props.selection.selected.map(tag => ({ ...tag })),
    retained: [...props.selection.retained], unresolved: [...props.selection.unresolved],
  } : emptyPublishTags()
}, { immediate: true })
const filtered = computed(() => props.candidates.filter(tag => tag.name.includes(search.value.trim())))
const count = computed(() => publishTagNames(temporary.value).length)
function searchInput(event: unknown) {
  search.value = (event as { detail: { value: string } }).detail.value
}
function toggle(tag: HomeMenuTag) {
  if (props.disabled) return
  notice.value = ''
  const index = temporary.value.selected.findIndex(item => item.id === tag.id)
  if (index >= 0) temporary.value.selected.splice(index, 1)
  else if (count.value >= 10) notice.value = '最多选择 10 个标签'
  else temporary.value.selected.push({ id: tag.id, name: tag.name })
}
function close() { uni.hideKeyboard(); emit('close') }
function confirm() {
  if (props.disabled || temporary.value.unresolved.length || count.value > 10) return
  uni.hideKeyboard()
  emit('confirm', temporary.value)
}
</script>
<template>
  <page-container :show="open" position="bottom" :round="true" :overlay="true" :duration="0" @beforeleave="close" @clickoverlay="close">
    <view v-if="open" class="tag-picker" :style="{ height: `calc(${keyboardHeight ? 90 : 70}vh - ${keyboardHeight}px)`, marginBottom: `${keyboardHeight}px` }" role="dialog" aria-label="选择标签">
      <view class="row spread tag-picker-head"><text class="heading">选择标签</text><text>已选 {{ count }}/10</text></view>
      <input :value="search" class="input" :disabled="disabled" :adjust-position="false" placeholder="搜索标签" aria-label="搜索标签" @input="searchInput" @keyboardheightchange="keyboardHeight = Math.max(0, $event.detail.height)" />
      <scroll-view scroll-y class="tag-picker-list">
        <view v-if="temporary.unresolved.length" class="tag-picker-section">
          <text class="error">待处理旧草稿标签不再支持提交，请确认移除后继续。</text>
          <button v-for="name in temporary.unresolved" :key="name" size="mini" :disabled="disabled" @tap="temporary.unresolved = temporary.unresolved.filter(item => item !== name)">{{ name }} · 确认移除</button>
        </view>
        <view v-if="temporary.retained.length" class="tag-picker-section">
          <text class="muted">历史标签仅可保留或移除，移除后不能重新添加。</text>
          <button v-for="name in temporary.retained" :key="name" size="mini" :disabled="disabled" @tap="temporary.retained = temporary.retained.filter(item => item !== name)">{{ name }} · 移除</button>
        </view>
        <view class="tag-picker-options">
          <button v-for="tag in filtered" :key="tag.id" class="tag-picker-option" :class="{ selected: temporary.selected.some(item => item.id === tag.id) }" :disabled="disabled" :aria-label="`${tag.name}，${temporary.selected.some(item => item.id === tag.id) ? '已选中' : '未选中'}`" @tap="toggle(tag)">
            <text>{{ temporary.selected.some(item => item.id === tag.id) ? '✓ ' : '' }}{{ tag.name }}</text><text v-if="tag.menu_status === 'hot'" class="tag-picker-hot"> 热门</text>
          </button>
        </view>
        <text v-if="!filtered.length" class="muted">{{ candidates.length ? '未找到标签' : '暂无可选标签' }}</text>
      </scroll-view>
      <text v-if="notice" class="error">{{ notice }}</text>
      <view class="row tag-picker-actions">
        <button @tap="close">取消</button>
        <button class="primary" :disabled="disabled || !!temporary.unresolved.length || count > 10" @tap="confirm">确定</button>
      </view>
    </view>
  </page-container>
</template>
<style scoped>
.tag-picker { box-sizing: border-box; display: flex; flex-direction: column; height: 70vh; padding: 24rpx 24rpx calc(24rpx + env(safe-area-inset-bottom)); background: #fff; }
.tag-picker-head, .tag-picker-actions { flex-shrink: 0; }
.tag-picker-list { flex: 1; min-height: 0; height: 0; }
.tag-picker-section { display: flex; flex-direction: column; gap: 16rpx; padding: 16rpx 0; }
.tag-picker-options { display: flex; flex-wrap: wrap; gap: 16rpx; padding: 16rpx 0; }
.tag-picker-option { max-width: 100%; margin: 0; font-size: 28rpx; white-space: normal; overflow-wrap: anywhere; }
.tag-picker-option.selected { color: var(--primary); border-color: currentColor; }
.tag-picker-hot { font-size: 22rpx; }
.tag-picker-actions > button { flex: 1; }
</style>
