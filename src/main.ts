import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { invoke } from '@tauri-apps/api/core'
import App from './App.vue'
import naive from 'naive-ui'
import router from './router'
import { installConsoleBridge, setDebugLogBridge } from './composables/useDebugLog'
import '@fontsource/nunito'
import '@material-design-icons/font/filled.css'
import './assets/nd-shared.css'

installConsoleBridge()

// Sync the console bridge with the persisted debug-logging toggle.
invoke<Record<string, unknown>>('get_all_settings')
  .then((s) => setDebugLogBridge(Boolean(s?.debugLogging)))
  .catch(() => {})

const app = createApp(App)
app.use(createPinia())
app.use(router)
app.use(naive)
app.mount('#app')
