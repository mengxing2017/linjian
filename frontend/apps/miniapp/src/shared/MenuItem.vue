<script setup lang="ts">
import AppIcon from './AppIcon.vue'
import type { IconName } from './icons'
withDefaults(defineProps<{ title: string; icon?: IconName; arrow?: boolean; disabled?: boolean; danger?: boolean; centered?: boolean }>(), { arrow: true, disabled: false, danger: false, centered: false })
const emit = defineEmits<{ activate: [] }>()
</script>
<template>
  <button class="menu-item" :class="{ danger, centered }" :disabled="disabled" @tap="!disabled && emit('activate')">
    <view v-if="icon" class="menu-item-icon" :class="`tone-${icon}`"><AppIcon :name="icon" :size="38" /></view>
    <text class="menu-item-title">{{ title }}</text>
    <AppIcon v-if="arrow" name="next" :size="28" />
  </button>
</template>
<style scoped>
.menu-item { display: flex; align-items: center; gap: 20rpx; width: 100%; min-height: 100rpx; margin: 0; padding: 16rpx 4rpx; box-sizing: border-box; line-height: 1.5; background: #fff; border-bottom: 1rpx solid #eef1f5; border-radius: 0; }
.menu-item::after { border: 0; }
.menu-item-icon { display: flex; align-items: center; justify-content: center; flex-shrink: 0; width: 68rpx; height: 68rpx; border-radius: 20rpx; background: #eef3ff; }
.menu-item-icon :deep(.app-icon) { margin-right: 0; }
.tone-join, .tone-business { background: #fff3e6; }
.tone-service, .tone-school, .tone-privacy { background: #eaf7f4; }
.menu-item-title { flex: 1; min-width: 0; text-align: left; white-space: normal; overflow-wrap: anywhere; }
.centered .menu-item-title { text-align: center; }
.danger { color: #bd303c; }
.menu-item[disabled] { opacity: .55; }
</style>
