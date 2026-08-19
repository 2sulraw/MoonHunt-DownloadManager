<template>
  <div class="cp-category" :class="{ enabled }">
    <div class="cp-row">
      <div class="cp-left">
        <i class="material-icons">category</i>
        <span>{{ t('Categories') }}</span>
      </div>
      <div class="cp-right">
        <button v-if="enabled" class="cp-manage" @click="managerRef?.open()">
          <i class="material-icons">folder_open</i>
          {{ t('Manage') }}
        </button>
        <div
          class="cp-toggle"
          :class="{ active: enabled }"
          @click="emit('update:enabled', !enabled)"
          role="switch"
          :aria-checked="enabled"
        >
          <span class="cp-toggle-track"><span class="cp-toggle-thumb"></span></span>
        </div>
      </div>
    </div>

    <div v-if="enabled" class="cp-controls">
      <CustomDropdown
        :model-value="mode"
        :options="modeOptions"
        @update:model-value="onModeChange"
      />

      <template v-if="mode === 'formatted' || mode === 'packaged'">
        <div class="cp-preview">
          <i class="material-icons">folder</i>
          <span>{{ autoPreview }}</span>
        </div>
      </template>

      <template v-else-if="mode === 'manual'">
        <CustomDropdown
          :model-value="selected"
          :options="manualCategoryOptions"
          @update:model-value="onSelectedChange"
        />
        <div v-if="!selected" class="cp-hint">{{ t('Select a saved category or open Manage to create one.') }}</div>
        <div v-else class="cp-preview">
          <i class="material-icons">folder</i>
          <span>{{ selected }}</span>
        </div>
      </template>
    </div>

    <CategoryManagerModal ref="managerRef" :selected="selected" @select="emit('update:selected', $event)" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import CategoryManagerModal from './CategoryManagerModal.vue'
import CustomDropdown from './CustomDropdown.vue'
import { useDownloadCategories } from '../composables/useDownloadCategories'
import { formattedCategoryFolder, packagedCategoryFolder, type DownloadCategoryMode } from '../utils/downloadCategories'
import { useI18n } from '../i18n'

const props = defineProps<{
  enabled: boolean
  mode: DownloadCategoryMode
  selected: string
  filename: string
}>()

const emit = defineEmits<{
  'update:enabled': [value: boolean]
  'update:mode': [value: DownloadCategoryMode]
  'update:selected': [value: string]
}>()
const { t } = useI18n()

const { categories } = useDownloadCategories()
const managerRef = ref<InstanceType<typeof CategoryManagerModal> | null>(null)

const modeOptions = [
  { value: 'formatted', label: t('Formatted') },
  { value: 'packaged', label: t('Packaged') },
  { value: 'manual', label: t('Manual') },
]

const manualCategoryOptions = computed(() => [
  { value: '', label: t('Choose a category...') },
  ...categories.value.map((category) => ({ value: category, label: category })),
])

const autoPreview = computed(() => {
  if (props.mode === 'packaged') {
    return `${t('Auto tree:')} ${packagedCategoryFolder(props.filename)}`
  }
  return `${t('Auto folder:')} ${formattedCategoryFolder(props.filename)}`
})

function onModeChange(value: string) {
  emit('update:mode', value as DownloadCategoryMode)
}

function onSelectedChange(value: string) {
  emit('update:selected', value)
}
</script>

<style scoped>
.cp-category {
  background: rgba(255, 255, 255, 0.02);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 12px 14px;
  transition: all 0.25s ease;
}

.cp-category.enabled {
  background: rgba(var(--accent-rgb), 0.04);
  border-color: rgba(var(--accent-rgb), 0.18);
}

.cp-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.cp-left {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-muted);
  font-size: 13px;
}

.cp-left .material-icons {
  font-size: 18px;
  color: var(--text-dim);
}

.cp-category.enabled .cp-left {
  color: var(--text);
}

.cp-category.enabled .cp-left .material-icons {
  color: var(--accent-strong);
}

.cp-right {
  display: flex;
  align-items: center;
  gap: 8px;
}

.cp-manage {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: none;
  border: none;
  color: var(--accent-strong);
  font-size: 11.5px;
  font-weight: 700;
  font-family: inherit;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 6px;
  transition: background 0.15s;
}

.cp-manage:hover {
  background: rgba(var(--accent-rgb), 0.1);
}

.cp-manage i {
  font-size: 14px;
}

.cp-toggle {
  width: 42px;
  height: 24px;
  border-radius: 12px;
  background: var(--hover-bg);
  border: 1px solid var(--border-strong);
  cursor: pointer;
  position: relative;
  flex-shrink: 0;
  transition: background 0.25s ease, border-color 0.25s ease;
}

.cp-toggle.active {
  background: rgba(var(--accent-rgb), 0.25);
  border-color: rgba(var(--accent-rgb), 0.4);
}

.cp-toggle-track {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  transition: transform 0.25s cubic-bezier(0.34, 1.4, 0.64, 1);
}

.cp-toggle.active .cp-toggle-track {
  transform: translateX(18px);
}

.cp-toggle-thumb {
  position: absolute;
  inset: 0;
  border-radius: 50%;
  background: var(--text-dim);
  transition: background 0.25s ease;
}

.cp-toggle.active .cp-toggle-thumb {
  background: var(--accent);
}

.cp-controls {
  margin-top: 11px;
  padding-top: 11px;
  border-top: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.cp-preview {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-muted);
}

.cp-preview i {
  font-size: 15px;
  color: var(--accent-strong);
}

.cp-hint {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--text-dim);
  padding: 0 4px;
}

.cp-hint::before {
  content: "info";
  font-family: "Material Icons";
  font-size: 13px;
  color: var(--accent-strong);
}
</style>
