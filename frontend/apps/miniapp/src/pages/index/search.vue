<script setup lang="ts">
import { schoolTheme } from '../../features/school-context/theme'
import { useProtectedScreen } from '../../app/page-lifecycle'
const { screen, allowed } = useProtectedScreen()
import Feed from '../../features/community/Feed.vue'
import Search from '../../features/community/Search.vue'
import { postSearchRoute } from '../../features/community/search-route'
import { ref } from 'vue'
import { onLoad } from '@dcloudio/uni-app'
const view = ref<'search' | 'mine' | 'collect'>('search')
onLoad((query) => {
  const route = postSearchRoute(query?.view)
  view.value = route.view
  uni.setNavigationBarTitle({ title: route.title })
})
</script>
<template>
  <view :style="schoolTheme">
    <Search v-if="allowed && view === 'search'" ref="screen" />
    <Feed
      v-else-if="allowed && view !== 'search'"
      ref="screen"
      :initial="view"
      :search-page="true"
    />
  </view>
</template>
