<script setup lang="ts">
interface Props {
  gid: string
}

defineProps<Props>()
</script>

<template>
  <div class="drag-item" :data-gid="gid">
    <slot />
  </div>
</template>

<style scoped>
.drag-item {
  user-select: none;
}

</style>
