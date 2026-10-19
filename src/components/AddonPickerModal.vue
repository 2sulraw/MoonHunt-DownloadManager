<template>
  <transition name="modal-fade">
    <div v-if="visible" class="nd-overlay" @mousedown.self="emit('close')">
      <div class="nd-modal addon-picker" @click.stop>
        <div class="nd-glow"></div>

        <header class="nd-header">
          <div class="nd-header-left">
            <div class="nd-avatar">
              <i class="material-icons">extension</i>
            </div>
            <div class="nd-titles">
              <h1 class="nd-title">{{ t('Add Addon') }}</h1>
              <span class="nd-subtitle">{{ t('Official, file, folder, or URL — community-made works too.') }}</span>
            </div>
          </div>
          <button class="nd-icon-btn nd-close" @click="emit('close')" :aria-label="t('Close')">
            <i class="material-icons">close</i>
          </button>
        </header>

        <div class="nd-divider"></div>

        <div class="nd-body">
          <div class="addon-trust">
            <i class="material-icons">warning</i>
            <span>{{ t('Only install addons you trust. Addons run inside MoonHunt with full app privileges.') }}</span>
          </div>

          <h3 class="nd-section-title">{{ t('Community addons') }}</h3>
          <p class="nd-field-hint">{{ t('Install a .zip package from a file, a folder, or a link. A playlist-bridge example is preinstalled.') }}</p>
          <div class="addon-actions">
            <button class="nd-btn nd-btn-secondary" @click="pickFile" :disabled="busy">
              <i class="material-icons">folder_open</i>
              {{ t('Choose .zip file') }}
            </button>
            <button class="nd-btn nd-btn-secondary" @click="pickFolder" :disabled="busy">
              <i class="material-icons">drive_folder_upload</i>
              {{ t('Install from folder') }}
            </button>
          </div>
          <div class="nd-input-wrap addon-url-wrap">
            <input class="nd-input" v-model="urlInput" placeholder="https://example.com/my-addon.zip" @keyup.enter="installUrl" />
          </div>
          <button class="nd-btn nd-btn-secondary addon-url-btn" @click="installUrl" :disabled="busy || !urlInput.trim()">
            <i class="material-icons">download</i>
            {{ t('Install from URL') }}
          </button>

          <h3 class="nd-section-title">{{ t('Make your own') }}</h3>
          <p class="nd-field-hint">{{ t('Get the starter template (manifest.json + addon.js + guide), edit it, test via Install from folder, share as .zip.') }}</p>
          <button class="nd-btn nd-btn-secondary" @click="getTemplate" :disabled="busy">
            <i class="material-icons">code</i>
            {{ t('Download starter template') }}
          </button>

          <div v-if="busy" class="spm-box spm-ok">
            <span class="media-spinner"></span>
            <span>{{ t('Installing...') }}</span>
          </div>
          <div v-if="error" class="spm-box spm-err">
            <i class="material-icons">error</i>
            <span>{{ error }}</span>
          </div>
          <div v-if="success" class="spm-box spm-ok">
            <i class="material-icons">check_circle</i>
            <span>{{ success }}</span>
          </div>
        </div>

        <footer class="nd-footer">
          <button class="nd-btn nd-btn-secondary" @click="emit('close')">
            {{ t('Close') }}
          </button>
        </footer>
      </div>
    </div>
  </transition>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import { useI18n } from '../i18n'
import { useAddons } from '../composables/useAddons'

const props = defineProps<{ visible: boolean }>()
const emit = defineEmits<{ close: []; installed: [id: string] }>()

const { t } = useI18n()
const {
  refresh,
  installFromPath, installFromDir, installFromUrl, exportTemplate,
} = useAddons()

const urlInput = ref('')
const busy = ref(false)
const error = ref('')
const success = ref('')

watch(() => props.visible, (v) => {
  if (v) {
    error.value = ''
    success.value = ''
    void refresh()
  }
})

async function run<T>(fn: () => Promise<T>, ok: (v: T) => void) {
  busy.value = true
  error.value = ''
  success.value = ''
  try {
    ok(await fn())
  } catch (e) {
    error.value = String(e)
  } finally {
    busy.value = false
  }
}

async function pickFile() {
  void (async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'Addon package', extensions: ['zip'] }],
      title: t('Choose addon file'),
    }).catch((e) => {
      error.value = String(e)
      return null
    })
    if (!selected) return
    void run(
      () => installFromPath(selected as string),
      (a) => {
        success.value = `${t('Addon added')}: ${a.manifest.name} v${a.manifest.version}`
        emit('installed', a.manifest.id)
      },
    )
  })()
}

function pickFolder() {
  void (async () => {
    const selected = await open({
      multiple: false,
      directory: true,
      title: t('Choose addon folder'),
    }).catch((e) => {
      error.value = String(e)
      return null
    })
    if (!selected) return
    void run(
      () => installFromDir(selected as string),
      (a) => {
        success.value = `${t('Addon added')}: ${a.manifest.name} v${a.manifest.version}`
        emit('installed', a.manifest.id)
      },
    )
  })()
}

function installUrl() {
  if (!urlInput.value.trim() || busy.value) return
  const u = urlInput.value.trim()
  void run(
    () => installFromUrl(u),
    (a) => {
      success.value = `${t('Addon added')}: ${a.manifest.name} v${a.manifest.version}`
      urlInput.value = ''
      emit('installed', a.manifest.id)
    },
  )
}

function getTemplate() {
  void (async () => {
    const dir = await open({
      multiple: false,
      directory: true,
      title: t('Choose empty folder for the template'),
    }).catch((e) => {
      error.value = String(e)
      return null
    })
    if (!dir) return
    void run(
      () => exportTemplate(dir as string),
      (msg) => {
        success.value = msg
      },
    )
  })()
}
</script>

<style scoped>
.addon-picker {
  width: min(520px, 100%);
}
.addon-trust {
  display: flex;
  gap: 8px;
  align-items: flex-start;
  padding: 10px 12px;
  border-radius: 8px;
  background: rgba(var(--accent-rgb), 0.08);
  color: var(--text);
  font-size: 12px;
  margin-bottom: 8px;
}
.addon-trust i {
  font-size: 18px;
  color: var(--accent-strong);
  flex-shrink: 0;
}
.addon-actions {
  display: flex;
  gap: 8px;
  margin-bottom: 8px;
}
.addon-actions .nd-btn {
  flex: 1;
}
.addon-url-wrap {
  margin-bottom: 8px;
}
.addon-url-btn {
  margin-bottom: 8px;
}
</style>
