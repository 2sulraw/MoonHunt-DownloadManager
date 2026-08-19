<template>
  <transition name="modal-fade">
    <div v-if="visible" class="about-modal-overlay" @mousedown.self.prevent="overlayPress()" @mouseup.self="overlayRelease() && $emit('close')" @mouseleave="overlayCancel()">
      <div class="about-modal">
        <button class="about-modal-close" @click="$emit('close')" :aria-label="t('Close')">
          <i class="material-icons">close</i>
        </button>
        <div class="ab-hero">
          <div class="ab-logo-ring">
            <i class="material-icons">nightlight_round</i>
          </div>
          <h2 class="ab-name">MoonHunt</h2>
          <span class="ab-ver" dir="ltr">v{{ version || '…' }}</span>
          <p class="ab-desc">{{ t('A polished desktop download manager built for speed, structure, and a calm experience.') }}</p>
          <div class="ab-tech">
            <span class="ab-chip">Tauri 2</span>
            <span class="ab-chip">Vue 3</span>
            <span class="ab-chip">TypeScript</span>
            <span class="ab-chip">aria2</span>
          </div>
        </div>
        <div class="ab-github">
          <button type="button" class="ab-github-card" @click="openGithub">
            <span class="ab-github-icon" aria-hidden="true">
              <LogoGithub class="ab-github-logo" />
            </span>
            <span class="ab-github-text">
              <strong>{{ t('View on GitHub') }}</strong>
              <span class="ab-github-url" dir="ltr">github.com/2sulraw/MoonHunt-DownloadManager</span>
            </span>
            <span class="ab-github-arrow"><i class="material-icons">open_in_new</i></span>
          </button>
        </div>
        <div class="ab-update">
          <button
            type="button"
            class="ab-update-card"
            @click="checkUpdate"
            :disabled="check.status === 'checking'"
          >
            <span class="ab-github-icon" aria-hidden="true">
              <i class="material-icons">system_update_alt</i>
            </span>
            <span class="ab-github-text">
              <strong>{{ check.status === 'checking' ? t('Checking for updates...') : t('Check for Update') }}</strong>
              <span v-if="check.status === 'upToDate'" class="ab-update-msg up">✓ {{ t('You are up to date') }}</span>
              <span v-else-if="check.status === 'available'" class="ab-update-msg avail">
                {{ t('Update available:') }}
                <a href="#" class="ab-update-link" dir="ltr" @click.prevent="openUpdate">{{ check.version }}</a>
              </span>
              <span v-else-if="check.status === 'noReleases'" class="ab-update-msg">{{ t('No releases yet') }}</span>
              <span v-else-if="check.status === 'error'" class="ab-update-msg error">{{ check.error || t('Could not check for updates') }}</span>
            </span>
            <span class="ab-github-arrow" v-if="check.status !== 'checking'"><i class="material-icons">chevron_right</i></span>
          </button>
        </div>
        <div class="ab-footer">
          {{ t('Built by') }} <strong>2sulraw</strong>
        </div>
      </div>
    </div>
  </transition>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { LogoGithub } from '@vicons/ionicons5'
import { useOverlayClose } from '../composables/useOverlayClose'
import { useI18n } from '../i18n'
defineProps<{ visible: boolean }>()
const emit = defineEmits<{ close: []; 'update-available': [info: { version?: string; url?: string; assetUrl?: string }] }>()

const { overlayPress, overlayCancel, overlayRelease } = useOverlayClose()
const { t } = useI18n()

const GITHUB_URL = 'https://github.com/2sulraw/MoonHunt-DownloadManager'

const version = ref('')
type CheckStatus = 'idle' | 'checking' | 'upToDate' | 'available' | 'noReleases' | 'error'
const check = ref<{ status: CheckStatus; version?: string; url?: string; error?: string }>({ status: 'idle' })

onMounted(() => {
  invoke<string>('get_app_version').then(v => {
    version.value = v
  }).catch(() => {})
})

async function checkUpdate() {
  if (check.value.status === 'checking') return
  check.value = { status: 'checking' }
  try {
    const res = await invoke<{ status: CheckStatus; version?: string; url?: string; assetUrl?: string }>('check_for_update')
    check.value = { status: res.status, version: res.version, url: res.url }
    if (res.status === 'available') {
      emit('update-available', { version: res.version, url: res.url, assetUrl: res.assetUrl })
    }
  } catch (e) {
    check.value = { status: 'error', error: typeof e === 'string' ? e : t('Could not check for updates') }
  }
}

function openUpdate() {
  if (check.value.url) {
    invoke('open_external', { url: check.value.url }).catch(() => {})
  }
}

function openGithub() {
  invoke('open_external', { url: GITHUB_URL }).catch(() => {
    window.open(GITHUB_URL, '_blank', 'noopener')
  })
}
</script>

<style scoped>
.about-modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
  backdrop-filter: blur(4px);
}

.about-modal {
  position: relative;
  width: min(400px, 90vw);
  max-height: 85vh;
  overflow-y: auto;
  border-radius: 16px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  box-shadow: var(--shadow-lg);
}

.about-modal-close {
  position: absolute;
  top: 12px;
  right: 12px;
  width: 28px;
  height: 28px;
  background: var(--hover-bg);
  border: none;
  border-radius: 6px;
  color: var(--text-dim);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
  z-index: 1;
}

.about-modal-close:hover {
  color: var(--text);
  background: var(--hover-bg);
}

.about-modal-close .material-icons { font-size: 16px; }

.ab-hero {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 28px 24px 16px;
  gap: 2px;
}

.ab-logo-ring {
  width: 52px;
  height: 52px;
  border-radius: 50%;
  background: rgba(var(--accent-rgb), 0.08);
  border: 1.5px solid rgba(var(--accent-rgb), 0.20);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 6px;
}

.ab-logo-ring .material-icons {
  font-size: 26px;
  color: var(--accent-strong);
}

.ab-name {
  margin: 0;
  font-size: 20px;
  font-weight: 700;
  letter-spacing: -0.01em;
}

.ab-ver {
  font-size: 10px;
  color: var(--text-dim);
  background: var(--hover-bg);
  padding: 1px 8px;
  border-radius: 999px;
  border: 1px solid var(--border);
  margin-top: 3px;
}

.ab-desc {
  color: var(--text-muted);
  font-size: 12.5px;
  line-height: 1.55;
  margin: 6px 0 0;
  max-width: 300px;
}

.ab-tech {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
  justify-content: center;
  padding: 10px 24px 0;
}

.ab-chip {
  padding: 3px 9px;
  border-radius: 5px;
  background: rgba(var(--accent-rgb), 0.06);
  color: var(--accent-strong);
  font-size: 11px;
  font-weight: 600;
  border: 1px solid rgba(var(--accent-rgb), 0.10);
}

.ab-github {
  padding: 12px 16px;
}

.ab-github-card {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 12px;
  background: var(--hover-bg);
  border: 1px solid var(--border);
  color: inherit;
  font: inherit;
  text-align: start;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
}

.ab-github-card:hover {
  background: rgba(var(--accent-rgb), 0.06);
  border-color: rgba(var(--accent-rgb), 0.28);
}

.ab-github-icon {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  background: rgba(var(--accent-rgb), 0.06);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  color: var(--accent-strong);
}

.ab-github-icon .material-icons {
  font-size: 16px;
  color: var(--accent-strong);
}

.ab-github-logo {
  width: 18px;
  height: 18px;
}

.ab-github-text {
  min-width: 0;
  flex: 1;
}

.ab-github-text strong {
  display: block;
  color: var(--text-muted);
  font-size: 12.5px;
  font-weight: 600;
  margin-bottom: 1px;
}

.ab-github-url {
  color: var(--text-dim);
  font-size: 11.5px;
  line-height: 1.45;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ab-github-arrow {
  color: var(--text-dim);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.ab-github-arrow .material-icons {
  font-size: 16px;
}

.ab-update {
  padding: 0 16px 4px;
}

.ab-update-card {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 12px;
  background: var(--hover-bg);
  border: 1px solid var(--border);
  color: inherit;
  font: inherit;
  text-align: start;
  cursor: pointer;
  transition: border-color 0.15s, background 0.15s;
}

.ab-update-card.checking {
  cursor: default;
  opacity: 0.7;
}

.ab-update-card:hover:not([disabled]) {
  background: rgba(var(--accent-rgb), 0.06);
  border-color: rgba(var(--accent-rgb), 0.28);
}

.ab-update-card[disabled] .ab-github-icon {
  color: var(--text-dim);
}

.ab-update-msg {
  font-size: 11.5px;
  line-height: 1.45;
  color: var(--text-dim);
}

.ab-update-msg.up { color: var(--status-success); }
.ab-update-msg.avail { color: var(--accent-strong); }
.ab-update-msg.error { color: var(--status-error); }

.ab-update-link {
  color: var(--accent-strong);
  font-weight: 600;
  text-decoration: none;
}

.ab-update-link:hover {
  text-decoration: underline;
}

.ab-footer {
  padding: 12px 20px;
  border-top: 1px solid var(--border);
  color: var(--text-dim);
  font-size: 12px;
  text-align: center;
}

.ab-footer strong { color: var(--text-muted); }
</style>
