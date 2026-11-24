<script setup lang="ts">
import { useProtectedScreen } from '../../app/page-lifecycle'
import Screen from '../../features/operations/Services.vue'
const { screen, allowed } = useProtectedScreen()
</script>
<template><Screen v-if="allowed" ref="screen" /></template>
