<template>
  <div class="toast-container">
    <transition-group name="fade">
      <div
        v-for="notif in notifications"
        :key="notif.id"
        class="toast"
      >
        <div class="toast-message">{{ notif.message }}</div>
        <div
          class="toast-progress"
          :style="{
            background: notif.type === 'success' ? 'var(--accent)' : notif.type === 'error' ? '#ef5350' : '#888',
          }"
        />
      </div>
    </transition-group>
  </div>
</template>

<script setup lang="ts">
import { useNotification } from '../composables/useNotification'

const { notifications } = useNotification()
</script>

<style scoped>
.toast-container {
  position: fixed;
  bottom: 142px;
  left: 50%;
  transform: translateX(-50%);
  padding: 16px;
  pointer-events: none;
}

.toast {
  background: var(--bg-card);
  color: var(--text);
  padding: 12px 20px;
  border-radius: 8px;
  margin-bottom: 8px;
  width: 320px;
  display: flex;
  flex-direction: column;
  font-size: 14px;
  font-weight: 500;
  pointer-events: auto;
  box-shadow: var(--shadow-md);
  border: 1px solid var(--border-strong);
  overflow: hidden;
  position: relative;
}

.toast-progress {
  position: absolute;
  bottom: 0;
  left: 0;
  height: 4px;
  animation: notificationProgress 3s linear forwards;
  width: 100%;
}

@keyframes notificationProgress {
  from { width: 100%; }
  to { width: 0%; }
}
</style>
