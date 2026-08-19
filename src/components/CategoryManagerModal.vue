<template>
  <transition name="cm-fade">
    <div v-if="isOpen" class="cm-overlay" @mousedown.self.prevent="overlayPress()" @mouseup.self="overlayRelease() && close()" @mouseleave="overlayCancel()">
      <div class="cm-panel">
        <div class="cm-head">
          <div class="cm-head-left">
            <i class="material-icons cm-head-icon">folder_special</i>
            <span class="cm-head-title">{{ t('Manual Categories') }}</span>
          </div>
          <button class="cm-close" @click="close" :aria-label="t('Close')">
            <i class="material-icons">close</i>
          </button>
        </div>

        <div class="cm-divider" />

        <div class="cm-body">
          <div class="cm-add-row">
            <input
              ref="addInputRef"
              v-model="newCategory"
              class="cm-input"
              :placeholder="t('New category folder...')"
              @keyup.enter="addCategory"
            />
            <button class="cm-add-btn" :disabled="!newCategory.trim()" @click="addCategory">
              <i class="material-icons">add</i>
              {{ t('Add') }}
            </button>
          </div>

          <div v-if="categories.length" class="cm-list">
            <div v-for="category in categories" :key="category" class="cm-item" :class="{ active: category === selected }">
              <template v-if="editingName === category">
                <input
                  v-model="editingValue"
                  class="cm-input cm-rename-input"
                  @keyup.enter="saveRename"
                  @keyup.esc="editingName = ''"
                />
                <button class="cm-icon-btn cm-save" :title="t('Save rename')" @click="saveRename">
                  <i class="material-icons">check</i>
                </button>
                <button class="cm-icon-btn" :title="t('Cancel rename')" @click="editingName = ''">
                  <i class="material-icons">close</i>
                </button>
              </template>
              <template v-else>
                <span class="cm-item-name" :title="category">
                  <i class="material-icons">folder</i>
                  {{ category }}
                </span>
                <div class="cm-item-actions">
                  <button class="cm-select-btn" @click="selectCategory(category)">{{ t('Select') }}</button>
                  <button class="cm-icon-btn" :title="t('Rename')" @click="startRename(category)">
                    <i class="material-icons">edit</i>
                  </button>
                  <button class="cm-icon-btn cm-danger" :title="t('Delete')" @click="deleteCategory(category)">
                    <i class="material-icons">delete</i>
                  </button>
                </div>
              </template>
            </div>
          </div>
          <div v-else class="cm-empty">
            <i class="material-icons">create_new_folder</i>
            <span>{{ t('Create your first category folder above.') }}</span>
          </div>
        </div>
      </div>
    </div>
  </transition>
</template>

<script setup lang="ts">
import { ref, nextTick } from 'vue'
import { useOverlayClose } from '../composables/useOverlayClose'
import { loadDownloadCategories, useDownloadCategories } from '../composables/useDownloadCategories'
import { useI18n } from '../i18n'

const props = defineProps<{
  selected?: string
}>()

const emit = defineEmits<{
  select: [name: string]
}>()

const { overlayPress, overlayCancel, overlayRelease } = useOverlayClose()
const { t } = useI18n()
const { categories, createCategory, renameCategory, removeCategory } = useDownloadCategories()

const isOpen = ref(false)
const newCategory = ref('')
const editingName = ref('')
const editingValue = ref('')
const addInputRef = ref<HTMLInputElement | null>(null)

function open() {
  isOpen.value = true
  newCategory.value = ''
  editingName.value = ''
  editingValue.value = ''
  void loadDownloadCategories()
  nextTick(() => addInputRef.value?.focus())
}

function close() {
  isOpen.value = false
}

function addCategory() {
  if (!newCategory.value.trim()) return
  const created = createCategory(newCategory.value)
  if (created) {
    emit('select', created)
    newCategory.value = ''
  }
}

function selectCategory(category: string) {
  emit('select', category)
  close()
}

function startRename(category: string) {
  editingName.value = category
  editingValue.value = category
}

function saveRename() {
  if (!editingName.value) return
  const renamed = renameCategory(editingName.value, editingValue.value)
  if (renamed) {
    if (props.selected === editingName.value) emit('select', renamed)
    editingName.value = ''
  }
}

function deleteCategory(category: string) {
  if (!confirm(`Delete category folder "${category}"?`)) return
  if (removeCategory(category) && props.selected === category) emit('select', '')
}

defineExpose({ open })
</script>

<style scoped>
.cm-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(6px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 10010;
}

.cm-panel {
  background: var(--bg-card);
  border: 1px solid var(--border-strong);
  border-radius: 14px;
  width: 320px;
  max-width: 92vw;
  max-height: 80vh;
  display: flex;
  flex-direction: column;
  box-shadow: var(--shadow-lg);
  animation: cm-pop 0.22s cubic-bezier(0.16, 1, 0.3, 1);
}

@keyframes cm-pop {
  from { opacity: 0; transform: scale(0.95) translateY(6px); }
  to { opacity: 1; transform: scale(1) translateY(0); }
}

.cm-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 14px;
}

.cm-head-left { display: flex; align-items: center; gap: 7px; }
.cm-head-icon { font-size: 17px; color: var(--accent-strong); }
.cm-head-title { font-size: 13px; font-weight: 700; color: var(--text); }
.cm-close {
  width: 24px;
  height: 24px;
  border-radius: 5px;
  background: transparent;
  border: none;
  color: var(--text-dim);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.12s, color 0.12s;
}
.cm-close:hover { background: var(--hover-bg); color: var(--text); }
.cm-close i { font-size: 15px; }

.cm-divider { height: 1px; background: var(--border); }

.cm-body { padding: 12px 14px; display: flex; flex-direction: column; gap: 10px; overflow-y: auto; }

.cm-add-row { display: flex; gap: 6px; }
.cm-input {
  flex: 1;
  min-width: 0;
  height: 30px;
  background: rgba(255, 255, 255, 0.03);
  border: 1px solid var(--border);
  border-radius: 7px;
  color: var(--text);
  padding: 0 10px;
  font-size: 12px;
  font-family: inherit;
  outline: none;
  box-sizing: border-box;
}
.cm-input:focus { border-color: var(--accent-strong); background: rgba(var(--accent-rgb), 0.03); }
.cm-input::placeholder { color: var(--text-dim); }

.cm-add-btn {
  height: 30px;
  border: none;
  border-radius: 7px;
  background: var(--accent);
  color: #0a0e1a;
  font-size: 11px;
  font-weight: 700;
  font-family: inherit;
  cursor: pointer;
  display: inline-flex;
  align-items: center;
  gap: 3px;
  padding: 0 10px;
}
.cm-add-btn:disabled { opacity: 0.35; cursor: not-allowed; }
.cm-add-btn i { font-size: 14px; }

.cm-list { display: flex; flex-direction: column; gap: 6px; }
.cm-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 7px 9px;
  background: var(--hover-bg);
  border: 1px solid transparent;
  border-radius: 8px;
  transition: border-color 0.12s, background 0.12s;
}
.cm-item.active { border-color: rgba(var(--accent-rgb), 0.35); background: rgba(var(--accent-rgb), 0.06); }
.cm-item-name {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12px;
  font-weight: 600;
  color: var(--text);
}
.cm-item-name i { font-size: 15px; color: var(--accent-strong); flex-shrink: 0; }
.cm-item-actions { display: flex; align-items: center; gap: 3px; flex-shrink: 0; }
.cm-select-btn {
  height: 24px;
  border: none;
  border-radius: 6px;
  background: rgba(var(--accent-rgb), 0.12);
  color: var(--accent-strong);
  font-size: 10.5px;
  font-weight: 700;
  font-family: inherit;
  cursor: pointer;
  padding: 0 8px;
}
.cm-select-btn:hover { background: rgba(var(--accent-rgb), 0.2); }
.cm-icon-btn {
  width: 24px;
  height: 24px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.12s, color 0.12s;
}
.cm-icon-btn:hover { background: var(--border-strong); color: var(--text); }
.cm-icon-btn i { font-size: 14px; }
.cm-danger:hover { background: rgba(239, 83, 80, 0.12); color: var(--status-error); }
.cm-save { color: var(--accent-strong); }

.cm-rename-input { flex: 1; }
.cm-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 7px;
  padding: 18px 10px;
  color: var(--text-dim);
  font-size: 12px;
  text-align: center;
}
.cm-empty i { font-size: 26px; color: var(--text-dim); opacity: 0.6; }

.cm-fade-enter-active,
.cm-fade-leave-active { transition: opacity 0.18s ease; }
.cm-fade-enter-from,
.cm-fade-leave-to { opacity: 0; }
</style>
