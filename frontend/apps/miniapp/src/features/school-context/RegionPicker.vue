<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { formatRegion, regionOptions } from '@linjian/china-regions'

const props = withDefaults(
  defineProps<{ province?: string; city?: string }>(),
  { province: '', city: '' },
)
const emit = defineEmits<{
  'update:province': [value: string]
  'update:city': [value: string]
}>()

const provinceIndex = ref(Math.max(0, regionOptions.findIndex((item) => item.name === props.province)))
const cityOptions = computed(() => regionOptions[provinceIndex.value]?.children ?? [])
const cityIndex = ref(Math.max(0, cityOptions.value.findIndex((item) => item.name === props.city)))
const range = computed(() => [
  regionOptions.map((item) => item.name),
  cityOptions.value.map((item) => item.name),
])
const value = computed(() => [provinceIndex.value, cityIndex.value])
const label = computed(() =>
  formatRegion({ province: props.province, city: props.city, area: '' }),
)

watch(
  () => [props.province, props.city],
  ([province, city]) => {
    const nextProvince = regionOptions.findIndex((item) => item.name === province)
    if (nextProvince < 0) return
    provinceIndex.value = nextProvince
    const nextCity = (regionOptions[nextProvince].children ?? []).findIndex(
      (item) => item.name === city,
    )
    cityIndex.value = nextCity < 0 ? 0 : nextCity
  },
)

function columnChange(event: { detail: { column: number; value: number } }) {
  if (event.detail.column === 0) {
    provinceIndex.value = event.detail.value
    cityIndex.value = 0
  } else {
    cityIndex.value = event.detail.value
  }
}

function change(event: { detail: { value: number[] } }) {
  const [nextProvince, nextCity] = event.detail.value
  provinceIndex.value = nextProvince
  cityIndex.value = nextCity
  const province = regionOptions[nextProvince]
  emit('update:province', province?.name ?? '')
  emit('update:city', province?.children?.[nextCity]?.name ?? '')
}

function clear() {
  provinceIndex.value = 0
  cityIndex.value = 0
  emit('update:province', '')
  emit('update:city', '')
}
</script>

<template>
  <view class="region-picker">
    <picker
      mode="multiSelector"
      :range="range"
      :value="value"
      @columnchange="columnChange"
      @change="change"
    >
      <view class="input">{{ label || '筛选城市' }}</view>
    </picker>
    <button v-if="city" class="region-clear" @tap="clear">清除</button>
  </view>
</template>

<style scoped>
.region-picker {
  display: flex;
  align-items: center;
  gap: 12rpx;
}
.region-picker picker {
  flex: 1;
}
.region-clear {
  flex: none;
  margin: 18rpx 0;
  padding: 0 20rpx;
  line-height: 72rpx;
  font-size: 26rpx;
}
</style>
