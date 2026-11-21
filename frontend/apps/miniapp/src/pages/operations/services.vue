<script setup lang="ts">
import { useScreen } from '../../app/page-lifecycle'
import Screen from '../../features/operations/Services.vue'
const screen = useScreen()
</script>
<template><Screen ref="screen" /></template>
