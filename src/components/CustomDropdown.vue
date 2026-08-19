<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue'

interface Option {
  value: string
  label: string
}

interface Props {
  modelValue: string
  options: Option[]
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

const isOpen = ref(false)
const dropdownRef = ref<HTMLElement | null>(null)

const selectedLabel = computed(() => {
  return props.options.find(o => o.value === props.modelValue)?.label ?? ''
})

function toggle() {
  isOpen.value = !isOpen.value
}

function select(value: string) {
  emit('update:modelValue', value)
  isOpen.value = false
}

function handleClickOutside(e: MouseEvent) {
  if (dropdownRef.value && !dropdownRef.value.contains(e.target as Node)) {
    isOpen.value = false
  }
}

onMounted(() => document.addEventListener('mousedown', handleClickOutside))
onUnmounted(() => document.removeEventListener('mousedown', handleClickOutside))
</script>

<template>
  <div class="custom-dropdown" ref="dropdownRef">
    <button class="custom-dropdown-trigger" @click="toggle" type="button">
      <span class="custom-dropdown-value">{{ selectedLabel }}</span>
      <svg class="custom-dropdown-arrow" :class="{ open: isOpen }" width="12" height="12" viewBox="0 0 12 12" fill="none">
        <path d="M3 5l3 3 3-3" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
    </button>
    <transition name="dd-slide">
      <div v-if="isOpen" class="custom-dropdown-list">
        <button
          v-for="opt in options"
          :key="opt.value"
          class="custom-dropdown-item"
          :class="{ active: modelValue === opt.value }"
          @click="select(opt.value)"
          type="button"
        >
          {{ opt.label }}
        </button>
      </div>
    </transition>
  </div>
</template>

<style scoped>
.custom-dropdown {
  position: relative;
  width: 100%;
}
.custom-dropdown-trigger {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  background: var(--hover-bg);
  border: 1px solid var(--border-strong);
  color: var(--text);
  padding: 10px 14px;
  border-radius: 10px;
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.2s ease;
  outline: none;
  box-sizing: border-box;
}
.custom-dropdown-trigger:hover {
  border-color: var(--border-strong);
  background: var(--toolbar-active);
}
.custom-dropdown-trigger:focus {
  border-color: var(--accent-strong);
  background: rgba(var(--accent-rgb), 0.03);
}
.custom-dropdown-value {
  flex: 1;
  text-align: left;
}
.custom-dropdown-arrow {
  color: var(--text-dim);
  flex-shrink: 0;
  transition: transform 0.2s ease;
}
.custom-dropdown-arrow.open {
  transform: rotate(180deg);
}
.custom-dropdown-list {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  background: var(--bg-card);
  border: 1px solid var(--border-strong);
  border-radius: 10px;
  padding: 4px;
  z-index: 100;
  box-shadow: var(--shadow-lg);
  max-height: 200px;
  overflow-y: auto;
}
.custom-dropdown-item {
  display: block;
  width: 100%;
  text-align: left;
  background: none;
  border: none;
  color: var(--text);
  padding: 8px 12px;
  border-radius: 7px;
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  transition: background 0.12s ease;
}
.custom-dropdown-item:hover {
  background: var(--hover-bg);
}
.custom-dropdown-item.active {
  background: rgba(var(--accent-rgb), 0.12);
  color: var(--accent-strong);
  font-weight: 600;
}
.dd-slide-enter-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}
.dd-slide-leave-active {
  transition: opacity 0.1s ease, transform 0.1s ease;
}
.dd-slide-enter-from,
.dd-slide-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
</style>
