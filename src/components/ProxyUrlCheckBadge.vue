<template>
  <span
    class="proxy-url-check"
    :class="{ direct: !proxyInfo?.configured, ok: result?.ok, fail: result && !result.ok, checking }"
    :title="title"
    :aria-label="title"
  >
    <span class="proxy-url-dot"></span>
    <template v-if="checking">
      <i class="material-icons proxy-url-spin">autorenew</i>
      <span>{{ t('Proxy') }}…</span>
    </template>
    <template v-else-if="!proxyInfo?.configured">
      <span>{{ t('Direct') }}</span>
    </template>
    <template v-else-if="!result">
      <span>{{ t('Proxy') }}</span>
    </template>
    <template v-else-if="result.ok">
      <span>{{ t('Proxy') }} · {{ result.status }} · {{ result.latencyMs }}ms</span>
    </template>
    <template v-else>
      <span>{{ t('Proxy') }} · {{ result.status > 0 ? result.status : t('Proxy test failed') }}</span>
    </template>
  </span>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { useI18n } from '../i18n'

interface ProxyInfo {
  configured: boolean
  proxy: string
  hasAuth: boolean
}

interface ProxyUrlCheckResult {
  ok: boolean
  status: number
  statusText?: string
  latencyMs: number
  contentLength?: number
  contentType?: string
  method?: string
  message?: string
  error?: string
  direct?: boolean
}

const props = defineProps<{
  url: string
  headers?: Record<string, string> | null
}>()

const { t } = useI18n()

const proxyInfo = ref<ProxyInfo | null>(null)
const checking = ref(false)
const result = ref<ProxyUrlCheckResult | null>(null)

let debounceTimer: ReturnType<typeof setTimeout> | undefined
let requestSequence = 0

const isHttpUrl = (value: string) => {
  try {
    const parsed = new URL(value)
    return parsed.protocol === 'http:' || parsed.protocol === 'https:'
  } catch {
    return false
  }
}

const title = computed(() => {
  if (!proxyInfo.value) return ''
  if (!proxyInfo.value.configured) return t('No proxy on aria2 — direct connection')
  const target = props.url.trim()
  if (checking.value) return `${t('Proxy')}: ${proxyInfo.value.proxy} · ${target}`
  if (!result.value) return `${t('Proxy')}: ${proxyInfo.value.proxy}`
  if (result.value.ok) {
    return result.value.message || `${t('Proxy connected')}: ${result.value.status} in ${result.value.latencyMs}ms`
  }
  return result.value.error || result.value.message || `${t('Proxy test failed')}: ${target}`
})

async function loadProxyInfo() {
  try {
    proxyInfo.value = await invoke<ProxyInfo>('check_aria2_proxy')
  } catch {
    proxyInfo.value = { configured: false, proxy: '', hasAuth: false }
  }
}

async function runCheck() {
  const target = props.url.trim()
  if (!proxyInfo.value?.configured || !target) {
    result.value = null
    checking.value = false
    return
  }

  const sequence = ++requestSequence
  checking.value = true
  result.value = null

  if (!isHttpUrl(target)) {
    result.value = {
      ok: false,
      status: 0,
      latencyMs: 0,
      error: 'Invalid URL: only http/https URLs can be checked through a proxy',
    }
    checking.value = false
    return
  }

  try {
    const check = await invoke<ProxyUrlCheckResult>('test_proxy_url', {
      url: target,
      headers: props.headers || undefined,
    })
    if (sequence !== requestSequence) return
    result.value = check
  } catch (e) {
    if (sequence !== requestSequence) return
    result.value = {
      ok: false,
      status: 0,
      latencyMs: 0,
      error: String(e),
    }
  } finally {
    if (sequence === requestSequence) checking.value = false
  }
}

function scheduleCheck() {
  clearTimeout(debounceTimer)
  const target = props.url.trim()
  if (!proxyInfo.value?.configured || !target) {
    result.value = null
    checking.value = false
    return
  }
  debounceTimer = setTimeout(() => void runCheck(), 350)
}

watch(() => props.url, scheduleCheck)
watch(() => props.headers, scheduleCheck)
watch(proxyInfo, scheduleCheck)

onMounted(async () => {
  await loadProxyInfo()
  scheduleCheck()
})

onBeforeUnmount(() => {
  clearTimeout(debounceTimer)
  requestSequence++
})
</script>

<style scoped>
.proxy-url-check {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  max-width: 190px;
  padding: 4px 8px;
  border-radius: 100px;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.03em;
  color: var(--accent-strong);
  background: rgba(var(--accent-rgb), 0.07);
  border: 1px solid rgba(var(--accent-rgb), 0.16);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.proxy-url-check.direct {
  color: var(--text-dim);
  background: var(--hover-bg);
  border-color: var(--border);
}
.proxy-url-check.ok {
  color: var(--status-complete);
  background: rgba(129, 199, 132, 0.08);
  border-color: rgba(129, 199, 132, 0.22);
}
.proxy-url-check.fail {
  color: var(--status-error);
  background: rgba(239, 83, 80, 0.08);
  border-color: rgba(239, 83, 80, 0.22);
}
.proxy-url-dot {
  width: 5px;
  height: 5px;
  border-radius: 50%;
  flex-shrink: 0;
  background: var(--accent);
  box-shadow: 0 0 4px rgba(var(--accent-rgb), 0.4);
}
.direct .proxy-url-dot {
  background: var(--text-dim);
  box-shadow: none;
}
.ok .proxy-url-dot {
  background: #81c784;
  box-shadow: 0 0 4px rgba(129, 199, 132, 0.5);
}
.fail .proxy-url-dot {
  background: #ef5350;
  box-shadow: 0 0 4px rgba(239, 83, 80, 0.5);
}
.proxy-url-spin {
  font-size: 12px;
  animation: proxy-url-spin 1s linear infinite;
}
@keyframes proxy-url-spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
