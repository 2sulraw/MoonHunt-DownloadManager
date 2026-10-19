<template>
  <div class="addon-host">
    <div v-if="loadError" class="spm-box spm-err">
      <i class="material-icons">error</i>
      <span>{{ loadError }}</span>
    </div>
    <div v-else-if="!loaded" class="spm-box spm-ok">
      <span class="media-spinner"></span>
      <span>{{ t('Loading addon...') }}</span>
    </div>
    <component
      v-else-if="remoteComponent"
      ref="remoteRef"
      :is="remoteComponent"
      :host="api"
      :initial-url="initialUrl"
    />
  </div>
</template>

<script setup lang="ts">
import { inject, nextTick, onBeforeUnmount, onErrorCaptured, onMounted, ref, watch } from 'vue'
import { useI18n } from '../i18n'
import { useAddons } from '../composables/useAddons'

const props = defineProps<{ addonId: string; initialUrl?: string }>()

const { t } = useI18n()
const notify = inject<(msg: string, type: 'success' | 'error' | 'info') => void>('notify', (m) => console.log(m))
const { readEntry, hostApi } = useAddons()

const api = hostApi(notify)
const loaded = ref(false)
const loadError = ref('')
const remoteComponent = ref<unknown>(null)
const remoteRef = ref<{ loadUrl?: (u: string) => void } | null>(null)
let objectUrl: string | null = null

onErrorCaptured((e) => {
  loadError.value = `${t('Addon crashed')}: ${String(e)}`
  return false
})

function revoke() {
  if (objectUrl) {
    URL.revokeObjectURL(objectUrl)
    objectUrl = null
  }
}

async function load() {
  loaded.value = false
  loadError.value = ''
  remoteComponent.value = null
  revoke()
  try {
    // Entry text comes from the backend; import via Blob URL so no extra
    // origin/CSP allowance is needed (blob: inherits the page origin).
    const code = await readEntry(props.addonId)
    const blob = new Blob([code], { type: 'text/javascript' })
    objectUrl = URL.createObjectURL(blob)
    const mod = (await import(/* @vite-ignore */ objectUrl)) as { default?: unknown }
    if (!mod?.default || (typeof mod.default !== 'object' && typeof mod.default !== 'function')) {
      throw new Error(t('Addon entry has no default export.'))
    }
    remoteComponent.value = mod.default
    await nextTick()
    forwardUrl(props.initialUrl)
  } catch (e) {
    loadError.value = `${t('Failed to load addon')}: ${String(e)}`
  } finally {
    loaded.value = true
  }
}

function forwardUrl(url: string | undefined) {
  if (url && remoteRef.value?.loadUrl) {
    try {
      remoteRef.value.loadUrl(url)
    } catch (e) {
      console.warn('Addon loadUrl failed:', e)
    }
  }
}

onMounted(() => void load())
onBeforeUnmount(() => revoke())
watch(() => props.addonId, () => void load())
watch(() => props.initialUrl, (v) => forwardUrl(v))
</script>

<style scoped>
.addon-host {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
</style>
