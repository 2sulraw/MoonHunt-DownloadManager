import { createRouter, createWebHistory, createWebHashHistory } from 'vue-router'
import MainLayout from '../layouts/MainLayout.vue'
import HomePage from '../views/HomePage.vue'
import DownloadsPage from '../views/DownloadsPage.vue'
import SettingsPage from '../views/SettingsPage.vue'
import MediaPage from '../views/Media.vue'
import AddonPage from '../views/AddonPage.vue'

const routes = [
  {
    path: '/',
    component: MainLayout,
    children: [
      { path: '', name: 'home', component: HomePage },
      { path: 'downloads', name: 'downloads', component: DownloadsPage },
      { path: 'media', name: 'media', component: MediaPage },
      { path: 'addon', name: 'addon', component: AddonPage },
      { path: 'settings', name: 'settings', component: SettingsPage },
    ]
  }
]

// Tauri in dev mode uses http://localhost:1420 (supports HTML5 history).
// Production builds use custom protocol (file://) — must use hash history.
const isTauri = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window
const router = createRouter({
  history: isTauri ? createWebHashHistory() : createWebHistory(),
  routes
})

export default router
