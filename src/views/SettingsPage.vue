<template>
  <div class="settings-layout">
    <aside class="settings-sidebar">
      <div class="sidebar-header">
        <h2 class="sidebar-title">{{ t('Settings') }}</h2>
      </div>
      <nav class="sidebar-nav">
        <button
          v-for="cat in categories"
          :key="cat.key"
          class="sidebar-item"
          :class="{ active: activeCategory === cat.key }"
          @click="activeCategory = cat.key"
        >
          <i class="material-icons">{{ cat.icon }}</i>
          <span>{{ t(cat.label) }}</span>
        </button>
      </nav>
      <div class="sidebar-footer">
        <button @click="saveSettings" class="sidebar-save-btn" :disabled="isSaving">
          <i class="material-icons" style="font-size:16px">save</i>
          {{ isSaving ? t('Saving...') : t('Save Changes') }}
        </button>
        <button @click="resetToDefaults" class="sidebar-reset-btn" :title="t('Reset to Defaults')">
          <i class="material-icons" style="font-size:16px">restart_alt</i>
          {{ t('Reset to Defaults') }}
        </button>
      </div>
    </aside>

    <main class="settings-content">
      <section v-show="activeCategory === 'general'" class="settings-section">
        <h2 class="section-title">{{ t('General') }}</h2>
        <p class="section-sub">{{ t('General application preferences.') }}</p>

        <div class="field">
          <label>{{ t('Language') }}</label>
          <div class="language-cards">
            <button
              v-for="localeInfo in LOCALES"
              :key="localeInfo.code"
              type="button"
              class="language-card"
              :class="{ active: language === localeInfo.code }"
              :aria-pressed="language === localeInfo.code"
              @click="onLanguageChange(localeInfo.code)"
            >
              <LanguageFlag :code="localeInfo.code" />
              <span class="language-name">{{ localeInfo.label }}</span>
              <i v-if="language === localeInfo.code" class="material-icons language-check">check_circle</i>
            </button>
          </div>
        </div>

        <div class="section-divider"></div>
        <h2 class="section-title" style="margin-top: 20px;">{{ t('Global Download Speed Limit') }}</h2>

        <div class="field row-field global-speed-limit">
          <label>{{ t('Max Download Speed') }}</label>
          <div class="speed-toggle-row">
            <div class="speed-toggle" :class="{ on: speedLimitEnabled }" @click="toggleSpeedLimit">
              <div class="speed-toggle-knob"></div>
            </div>
          </div>
        </div>
        <transition name="speed-slide">
          <div v-if="speedLimitEnabled" class="speed-control-row">
            <div class="stepper speed-stepper">
              <button @click="stepSpeedLimit(-1)" :disabled="speedLimitInputValue <= 0"><i class="material-icons">remove</i></button>
              <input v-model.number="speedLimitInputValue" type="number" min="0.01" step="0.01" @change="applySpeedInput" />
              <button @click="stepSpeedLimit(1)"><i class="material-icons">add</i></button>
            </div>
            <div class="speed-unit-switch" role="group" aria-label="Speed limit unit">
              <button
                v-for="unitOption in speedLimitUnitOptions"
                :key="unitOption.value"
                type="button"
                class="speed-unit-btn"
                :class="{ active: speedLimitUnit === unitOption.value }"
                @click="setSpeedLimitUnit(unitOption.value)"
              >
                {{ unitOption.label }}
              </button>
            </div>
          </div>
        </transition>
        <p class="field-hint">{{ t('This limit applies to all downloads at once.') }}</p>

        <div class="section-divider"></div>
        <h2 class="section-title" style="margin-top: 20px;">{{ t('Search') }}</h2>

        <div class="field row-field">
          <label>{{ t('Search Preview') }}</label>
          <div class="toggle-row">
            <div class="toggle" :class="{ active: searchPreviewEnabled }" @click="searchPreviewEnabled = !searchPreviewEnabled" role="switch" :aria-checked="searchPreviewEnabled">
              <div class="toggle-track"><div class="toggle-thumb"></div></div>
            </div>
          </div>
        </div>
        <p class="field-hint">{{ t('Show matching downloads in a dropdown while typing in the titlebar search.') }}</p>

        <div class="section-divider"></div>
        <h2 class="section-title" style="margin-top: 20px;">{{ t('Security') }}</h2>

        <div class="field row-field">
          <label>{{ t('Verify Certificates') }}</label>
          <div class="toggle-row">
            <div class="toggle" :class="{ active: verifySsl }" @click="verifySsl = !verifySsl" role="switch" :aria-checked="verifySsl">
              <div class="toggle-track"><div class="toggle-thumb"></div></div>
            </div>
          </div>
        </div>
        <p class="field-hint">{{ t('When enabled, certificates are verified during downloads. Disable only if you encounter certificate errors.') }}</p>
      </section>

      <section v-show="activeCategory === 'url'" class="settings-section">
        <h2 class="section-title">{{ t('URL Downloads') }}</h2>
        <p class="section-sub">{{ t('Where URL-based files land and how they download.') }}</p>

        <div class="field">
          <label>{{ t('Download Location') }}</label>
          <div class="nd-input-wrap">
            <input v-model="downloadLocation" class="nd-input nd-path-input" :placeholder="t('Default download location')" />
            <button @click="selectDownloadLocation" class="nd-input-action">
              <i class="material-icons">folder_open</i>
            </button>
          </div>
        </div>

        <div class="field">
          <label>{{ t('Max Concurrent Downloads') }}</label>
          <div class="stepper">
            <button @click="maxConcurrent = Math.max(1, maxConcurrent - 1)" :disabled="maxConcurrent <= 1"><i class="material-icons">remove</i></button>
            <input v-model.number="maxConcurrent" type="number" min="1" max="10" @change="maxConcurrent = Math.min(10, Math.max(1, Number(maxConcurrent) || 1))" />
            <button @click="maxConcurrent = Math.min(10, maxConcurrent + 1)" :disabled="maxConcurrent >= 10"><i class="material-icons">add</i></button>
          </div>
        </div>

        <div class="field">
          <label>{{ t('Retry Settings') }}</label>
          <p class="field-hint">{{ t('How aria2 retries on unstable connections.') }}</p>
        </div>

        <div class="retry-row">
          <div class="retry-field">
            <label>{{ t('Max Tries') }}</label>
            <div class="stepper">
              <button @click="maxRetries = Math.max(1, maxRetries - 1)" :disabled="maxRetries <= 1"><i class="material-icons">remove</i></button>
              <input v-model.number="maxRetries" type="number" min="1" max="999" @change="maxRetries = Math.min(999, Math.max(1, Number(maxRetries) || 5))" />
              <button @click="maxRetries = Math.min(999, maxRetries + 1)" :disabled="maxRetries >= 999"><i class="material-icons">add</i></button>
            </div>
          </div>

          <div class="retry-field">
            <label>{{ t('Retry per URI') }}</label>
            <div class="stepper">
              <button @click="retryPerUri = Math.max(1, retryPerUri - 1)" :disabled="retryPerUri <= 1"><i class="material-icons">remove</i></button>
              <input v-model.number="retryPerUri" type="number" min="1" max="999" @change="retryPerUri = Math.min(999, Math.max(1, Number(retryPerUri) || 1))" />
              <button @click="retryPerUri = Math.min(999, retryPerUri + 1)" :disabled="retryPerUri >= 999"><i class="material-icons">add</i></button>
            </div>
          </div>

          <div class="retry-field">
            <label>{{ t('Retry Wait (sec)') }}</label>
            <div class="stepper">
              <button @click="retryWait = Math.max(1, retryWait - 1)" :disabled="retryWait <= 1"><i class="material-icons">remove</i></button>
              <input v-model.number="retryWait" type="number" min="1" max="120" @change="retryWait = Math.min(120, Math.max(1, Number(retryWait) || 3))" />
              <button @click="retryWait = Math.min(120, retryWait + 1)" :disabled="retryWait >= 120"><i class="material-icons">add</i></button>
            </div>
          </div>
        </div>

        <div class="field">
          <div class="toggle-header-row">
            <label>{{ t('Enable Download Limit') }}</label>
            <div class="speed-toggle-row">
              <div class="speed-toggle" :class="{ on: dlLimitEnabled }" @click="toggleDlLimit">
                <div class="speed-toggle-knob"></div>
              </div>
            </div>
          </div>
        </div>

        <transition name="speed-slide">
          <div v-if="dlLimitEnabled" class="field">
            <label>{{ t('Limit') }}</label>
            <div class="nd-input-wrap">
              <input v-model.number="dlLimitInputValue" type="number" min="1" @input="syncBytesFromInput" class="nd-input nd-path-input" placeholder="e.g. 1" />
              <CustomDropdown
                v-model="dlLimitUnit"
                :options="dlLimitUnitOptions"
                class="dl-limit-unit-custom"
                @update:modelValue="syncBytesFromInput"
              />
            </div>
            <span class="field-hint">{{ t('Equivalent to') }} {{ formatLimitDisplay(dlLimitBytes) }}</span>
          </div>
        </transition>

        <div v-if="dlLimitEnabled && dlLimitBytes > 0" class="field">
          <label>{{ t('Usage') }}</label>
          <div class="limit-usage-bar">
            <div class="limit-usage-track">
              <div class="limit-usage-fill" :style="{ width: dlLimitUsagePercent + '%' }"></div>
            </div>
            <span class="limit-usage-text">{{ formatLimitDisplay(dlLimitUsedBytes) }} / {{ formatLimitDisplay(dlLimitBytes) }}</span>
          </div>
        </div>
      </section>

      <section v-show="activeCategory === 'torrent'" class="settings-section">
        <h2 class="section-title">{{ t('Torrent Downloads') }}</h2>
        <p class="section-sub">{{ t('Settings for BitTorrent downloads.') }}</p>

        <div class="field">
          <label>{{ t('Torrent Download Location') }}</label>
          <div class="nd-input-wrap">
            <input v-model="torrentDownloadLocation" class="nd-input nd-path-input" :placeholder="t('Default torrent download location')" />
            <button @click="selectTorrentDownloadLocation" class="nd-input-action">
              <i class="material-icons">folder_open</i>
            </button>
          </div>
        </div>

        <div class="field">
          <label>{{ t('Max Connections Per Torrent') }}</label>
          <div class="stepper">
            <button @click="torrentMaxConnections = Math.max(1, torrentMaxConnections - 1)" :disabled="torrentMaxConnections <= 1"><i class="material-icons">remove</i></button>
            <input v-model.number="torrentMaxConnections" type="number" min="1" max="128" @change="torrentMaxConnections = Math.min(128, Math.max(1, Number(torrentMaxConnections) || 1))" />
            <button @click="torrentMaxConnections = Math.min(128, torrentMaxConnections + 1)" :disabled="torrentMaxConnections >= 128"><i class="material-icons">add</i></button>
          </div>
        </div>

        <div class="field row-field">
          <label>{{ t('Enable DHT') }}</label>
          <div class="toggle-row">
            <div class="toggle" :class="{ active: torrentDhtEnabled }" @click="torrentDhtEnabled = !torrentDhtEnabled" role="switch" :aria-checked="torrentDhtEnabled">
              <div class="toggle-track"><div class="toggle-thumb"></div></div>
            </div>
          </div>
        </div>

        <div class="field row-field">
          <label>{{ t('Auto-Start Torrents') }}</label>
          <div class="toggle-row">
            <div class="toggle" :class="{ active: torrentAutoStart }" @click="torrentAutoStart = !torrentAutoStart" role="switch" :aria-checked="torrentAutoStart">
              <div class="toggle-track"><div class="toggle-thumb"></div></div>
            </div>
          </div>
        </div>

        <div class="field row-field global-speed-limit">
          <label>{{ t('Max Upload Speed') }}</label>
          <div class="speed-toggle-row">
            <div class="toggle" :class="{ active: torrentUploadLimitEnabled }" @click="torrentUploadLimitEnabled = !torrentUploadLimitEnabled; if (torrentUploadLimitEnabled) syncTorrentUploadInput()" role="switch" :aria-checked="torrentUploadLimitEnabled">
              <div class="toggle-track"><div class="toggle-thumb"></div></div>
            </div>
          </div>
        </div>
        <transition name="speed-slide">
          <div v-if="torrentUploadLimitEnabled" class="speed-control-row">
            <div class="stepper speed-stepper">
              <button @click="stepTorrentUploadLimit(-1)" :disabled="torrentUploadLimitInputValue <= 0.01"><i class="material-icons">remove</i></button>
              <input v-model.number="torrentUploadLimitInputValue" type="number" min="0.01" step="0.01" @change="applyTorrentUploadInput" />
              <button @click="stepTorrentUploadLimit(1)"><i class="material-icons">add</i></button>
            </div>
            <div class="speed-unit-switch" role="group" aria-label="Upload speed limit unit">
              <button
                v-for="unitOption in speedLimitUnitOptions"
                :key="unitOption.value"
                type="button"
                class="speed-unit-btn"
                :class="{ active: torrentUploadLimitUnit === unitOption.value }"
                @click="setTorrentUploadLimitUnit(unitOption.value)"
              >
                {{ unitOption.label }}
              </button>
            </div>
          </div>
        </transition>
        <p class="field-hint">{{ t('This limit applies to all torrent uploads at once.') }}</p>
      </section>

      <section v-show="activeCategory === 'network'" class="settings-section">
        <h2 class="section-title">{{ t('Proxy Configuration') }}</h2>
        <p class="section-sub">{{ t('Route traffic through a proxy server to bypass ISP restrictions.') }}</p>

        <div class="field row-field">
          <label>{{ t('Enable Proxy') }}</label>
          <div style="display: flex; align-items: center; gap: 8px;">
            <button @click="autoDetectProxy" class="btn-detect" :disabled="detectingProxy">
              <i class="material-icons" :style="{ fontSize: '16px', animation: detectingProxy ? 'spin 1s linear infinite' : 'none' }">{{ detectingProxy ? 'hourglass_empty' : 'wifi_find' }}</i>
              {{ detectingProxy ? t('Detecting…') : t('Auto Detect') }}
            </button>
            <div class="toggle-row">
              <div class="toggle" :class="{ active: proxyEnabled }" @click="proxyEnabled = !proxyEnabled; if (proxyEnabled && proxyType === 'none') proxyType = 'http'" role="switch" :aria-checked="proxyEnabled">
                <div class="toggle-track"><div class="toggle-thumb"></div></div>
              </div>
            </div>
          </div>
        </div>

        <div v-if="detectResult" class="detect-result" :class="{ ok: detectResult.enabled, fail: !detectResult.enabled }">
          <i class="material-icons">{{ detectResult.enabled ? 'check_circle' : 'info_outline' }}</i>
          {{ detectResult.message }}
        </div>

        <template v-if="proxyEnabled">
          <div class="field-row">
            <div class="field"><label>{{ t('Host') }}</label><input v-model="proxyHost" class="nd-input" :placeholder="t('proxy.example.com')" /></div>
            <div class="field"><label>{{ t('Port') }}</label><input v-model.number="proxyPort" type="number" class="nd-input" placeholder="1080" /></div>
          </div>

          <div class="field-row">
            <div class="field"><label>{{ t('Username') }} <span class="optional">{{ t('(optional)') }}</span></label><input v-model="proxyUser" class="nd-input" :placeholder="t('Leave blank if none')" /></div>
            <div class="field"><label>{{ t('Password') }} <span class="optional">{{ t('(optional)') }}</span></label><input v-model="proxyPass" type="password" class="nd-input" :placeholder="t('Leave blank if none')" /></div>
          </div>

          <!-- Proxy Checker -->
          <div class="proxy-checker">
            <button @click="testProxyConnection" class="btn-check" :disabled="proxyTesting">
              <i class="material-icons" :class="{ spinning: proxyTesting }">{{ proxyTestResult?.ok === true ? 'check_circle' : proxyTestResult?.ok === false ? 'error' : 'wifi_find' }}</i>
              {{ proxyTesting ? t('Testing…') : proxyTestResult ? (proxyTestResult.ok ? t('Re-test Connection') : t('Retry Connection')) : t('Test Connection') }}
            </button>
            <div v-if="proxyTestResult" class="proxy-test-result" :class="{ ok: proxyTestResult.ok, fail: !proxyTestResult.ok }">
              <span v-if="proxyTestResult.ok">
                <i class="material-icons">check_circle</i>
                {{ proxyTestResult.message }}
                <span v-if="proxyTestResult.ip"> · IP: {{ proxyTestResult.ip }}</span>
              </span>
              <span v-else>
                <i class="material-icons">error</i>
                {{ proxyTestResult.message }}
              </span>
            </div>
          </div>
        </template>

        <div v-else class="proxy-disabled-hint">
          <i class="material-icons">info_outline</i>
          <span>{{ t('Proxy is disabled. Enable it if your ISP blocks certain sites.') }}</span>
        </div>
      </section>

      <section v-show="activeCategory === 'network'" class="settings-section">
        <h2 class="section-title">{{ t('RPC Configuration') }}</h2>
        <p class="section-sub">{{ t('aria2 RPC server settings.') }}</p>

        <div class="field">
          <label>{{ t('RPC Port') }}</label>
          <div class="nd-input-wrap">
            <input v-model.number="rpcPort" type="number" min="1" max="65535" class="nd-input nd-path-input" />
            <button @click="copyToClipboard(rpcPort.toString())" class="nd-input-action" :title="t('Copy to clipboard')"><i class="material-icons">content_copy</i></button>
          </div>
        </div>
      </section>

      <section v-show="activeCategory === 'extension'" class="settings-section">
        <h2 class="section-title">{{ t('Browser Extension') }}</h2>
        <p class="section-sub">{{ t('Connect MoonHunt to your browser.') }}</p>

        <div class="field">
          <label>{{ t('Extension API Port') }}</label>
          <div class="nd-input-wrap">
            <input v-model.number="extensionPort" type="number" min="1" max="65535" class="nd-input nd-path-input" />
            <button @click="copyToClipboard(extensionPort.toString())" class="nd-input-action" :title="t('Copy to clipboard')"><i class="material-icons">content_copy</i></button>
          </div>
        </div>

        <div class="field row-field">
          <div>
            <label class="auth-token-label">
              <span>{{ t('Auth Token') }} <span class="optional">{{ extensionAuthEnabled ? t('(required when enabled)') : t('(optional)') }}</span></span>
              <button class="lamp-btn" :aria-label="t('About the auth token')">
                <i class="material-icons">lightbulb</i>
                <span class="lamp-tooltip">{{ t('Require a shared token so only your extension can connect. When enabled, requests without a matching token are rejected.') }}</span>
              </button>
            </label>
          </div>
          <div class="toggle-row">
            <div class="toggle" :class="{ active: extensionAuthEnabled }" @click="extensionAuthEnabled = !extensionAuthEnabled" role="switch" :aria-checked="extensionAuthEnabled">
              <div class="toggle-track"><div class="toggle-thumb"></div></div>
            </div>
          </div>
        </div>

        <div class="field">
          <div class="nd-input-wrap nd-input-wrap--token">
            <button @click="generateAuthToken" class="nd-input-lead" :title="t('Generate token')" :aria-label="t('Generate token')"><i class="material-icons">auto_fix_high</i></button>
            <input v-model="extensionAuthToken" :type="authTokenVisible ? 'text' : 'password'" class="nd-input nd-path-input nd-token-input" :placeholder="t('Enter a secret token')" />
            <button @click="authTokenVisible = !authTokenVisible" class="nd-input-action nd-hide-btn" :title="authTokenVisible ? t('Hide') : t('Show')" :aria-label="authTokenVisible ? t('Hide') : t('Show')"><i class="material-icons">{{ authTokenVisible ? 'visibility_off' : 'visibility' }}</i></button>
            <button @click="copyToClipboard(extensionAuthToken)" class="nd-input-action" :title="t('Copy to clipboard')" :aria-label="t('Copy to clipboard')"><i class="material-icons">content_copy</i></button>
          </div>
        </div>

        <div class="info-box">
          <div class="info-title">
            <i class="material-icons">info</i> {{ t('Browser Extension Setup') }}
          </div>
          <div class="tutorial-buttons">
            <button @click="openTutorial('chrome')" class="btn-tutorial">
              <i class="material-icons">lightbulb</i> {{ t('Chrome Tutorial') }}
            </button>
            <button @click="openFirefoxAddonPage" class="btn-tutorial">
              <i class="material-icons">lightbulb</i> {{ t('Firefox Tutorial') }}
            </button>
          </div>
        </div>

        <div class="field row-field">
          <div>
            <label>{{ t('Open MoonHunt from the browser') }}</label>
            <p class="field-hint">{{ t('Lets the extension wake MoonHunt when it is closed (moonhunt://).') }}</p>
          </div>
          <div class="toggle-row">
            <div class="toggle" :class="{ active: protocolRegistered }" @click="toggleProtocol" role="switch" :aria-checked="protocolRegistered">
              <div class="toggle-track"><div class="toggle-thumb"></div></div>
            </div>
          </div>
        </div>
      </section>

      <div v-if="showTutorial" class="tutorial-overlay" @click.self="showTutorial = false">
        <div class="tutorial-modal" :class="{ 'tutorial-modal-wide': isChrome }">
          <div class="tutorial-header">
            <i class="material-icons">lightbulb</i>
            <h3>{{ isChrome ? t('Chrome Tutorial') : t('Firefox Tutorial') }}</h3>
            <button class="tutorial-close" @click="showTutorial = false" :aria-label="t('Close')"><i class="material-icons">close</i></button>
          </div>
          <div class="tutorial-body">
            <template v-if="isChrome">
              <img v-if="tutorialPage === 1" src="/tutorials/chrome.gif" alt="Chrome extension tutorial" class="tutorial-gif" />
              <img v-else-if="tutorialPage === 2" src="/tutorials/chrome2.gif" alt="Chrome extension tutorial" class="tutorial-gif" />
              <img v-else src="/tutorials/chrome3.gif" alt="Chrome extension tutorial" class="tutorial-gif" />
              <ol v-if="tutorialPage === 1" class="info-steps">
                <li>{{ t('Open chrome://extensions and enable Developer mode.') }}</li>
              </ol>
              <ol v-else-if="tutorialPage === 2" class="info-steps" start="2">
                <li>{{ t('Now drag the extension zip folder into the extension page') }}</li>
              </ol>
              <ol v-else class="info-steps" start="3">
                <li>{{ t('Enable the Intercept') }}</li>
              </ol>
            </template>
            <template v-else>
              <ol class="info-steps">
                <li>{{ t('Open about:debugging → This Firefox.') }}</li>
                <li>{{ t('Click Load Temporary Add-on and select the manifest.json.') }}</li>
                <li>{{ t('Enable download interception and keep MoonHunt running.') }}</li>
              </ol>
            </template>
            <p class="tutorial-hint">{{ t('Downloads are captured automatically — no provider config needed.') }}</p>
          </div>
          <div class="tutorial-footer">
            <div v-if="isChrome" class="tutorial-pager">
              <button v-if="tutorialPage > 1" class="btn-config" @click="tutorialPage--">{{ t('Back') }}</button>
              <button v-if="tutorialPage < 3" class="btn-config" @click="tutorialPage++">{{ t('Next') }}</button>
            </div>
            <button v-if="!isChrome" @click="openTutorialRepo" class="btn-config">
              <i class="material-icons">open_in_new</i> {{ t('Open Extension Guide') }}
            </button>
          </div>
        </div>
      </div>


      <section v-show="activeCategory === 'appearance'" class="settings-section">
        <h2 class="section-title">{{ t('App Theme') }}</h2>
        <p class="section-sub">{{ t('Choose the overall visual style Application.') }}</p>

        <div class="theme-cards">
          <div
            class="theme-card"
            :class="{ active: localMode === 'dark' }"
            @click="localMode = 'dark'"
          >
            <div class="theme-card-preview dark-preview">
              <div class="preview-bar"></div>
              <div class="preview-body">
                <div class="preview-line"></div>
                <div class="preview-line short"></div>
              </div>
            </div>
            <div class="theme-card-info">
              <span class="theme-card-name">{{ t('Dark') }}</span>
              <span class="theme-card-desc">{{ t('Deep navy backgrounds') }}</span>
            </div>
            <i v-if="localMode === 'dark'" class="material-icons theme-card-check">check_circle</i>
          </div>

          <div
            class="theme-card"
            :class="{ active: localMode === 'light' }"
            @click="localMode = 'light'"
          >
            <div class="theme-card-preview light-preview">
              <div class="preview-bar light"></div>
              <div class="preview-body light">
                <div class="preview-line light"></div>
                <div class="preview-line short light"></div>
              </div>
            </div>
            <div class="theme-card-info">
              <span class="theme-card-name">{{ t('Light') }}</span>
              <span class="theme-card-desc">{{ t('Warm cream surfaces') }}</span>
            </div>
            <i v-if="localMode === 'light'" class="material-icons theme-card-check">check_circle</i>
          </div>

          <div
            class="theme-card"
            :class="{ active: localMode === 'linear-dark' }"
            @click="localMode = 'linear-dark'"
          >
            <div class="theme-card-preview linear-preview">
              <div class="preview-bar linear"></div>
              <div class="preview-body linear">
                <div class="preview-line linear"></div>
                <div class="preview-line short linear"></div>
              </div>
            </div>
            <div class="theme-card-info">
              <span class="theme-card-name">{{ t('Linear Dark') }}</span>
              <span class="theme-card-desc">{{ t('Near-black, minimal surfaces') }}</span>
            </div>
            <i v-if="localMode === 'linear-dark'" class="material-icons theme-card-check">check_circle</i>
          </div>
        </div>

        <div class="field" style="margin-top: 24px;">
          <label>{{ t('Accent Color') }}</label>
          <div class="color-swatches">
            <button v-for="theme in themeApi.THEMES" :key="theme.name" class="swatch" :class="{ active: localTheme === theme.name }" :style="{ background: theme.accent }" :title="t(theme.label)" :aria-label="t(theme.label)" @click="localTheme = theme.name">
              <i v-if="localTheme === theme.name" class="material-icons">check</i>
            </button>
          </div>
        </div>
      </section>

      <section v-show="activeCategory === 'performance'" class="settings-section">
        <h2 class="section-title">{{ t('Performance') }}</h2>
        <p class="section-sub">{{ t('App resource usage and storage.') }}</p>

        <div v-if="statsError" class="perf-error">
          <i class="material-icons">error_outline</i>
          <span>{{ statsError }}</span>
        </div>

        <div v-else class="perf-grid">
          <div class="perf-stat">
            <div class="perf-stat-header"><i class="material-icons perf-stat-icon">memory</i><span class="perf-stat-label">{{ t('Memory') }}</span></div>
            <div class="perf-stat-value">{{ stats.memory?.used_human || '—' }}</div>
            <div class="perf-stat-sub">{{ stats.memory?.percent?.toFixed(1) || '0' }}% {{ t('of') }} {{ stats.memory?.total_human || '—' }}</div>
            <div class="perf-bar-wrap"><div class="perf-bar" :style="{ width: (stats.memory?.percent || 0) + '%' }"></div></div>
            <div class="perf-stat-breakdown">
              <span>{{ t('System:') }} {{ stats.memory?.system_used_human || '—' }}</span>
              <span>{{ t('Total:') }} {{ stats.memory?.total_human || '—' }}</span>
            </div>
          </div>
          <div class="perf-stat">
            <div class="perf-stat-header"><i class="material-icons perf-stat-icon">developer_board</i><span class="perf-stat-label">{{ t('CPU') }}</span></div>
            <div class="perf-stat-value">{{ stats.cpu?.usage_percent || '0' }}%</div>
            <div class="perf-stat-sub">{{ stats.cpu?.process_percent || '0' }}% {{ t('app') }} · {{ stats.cpu?.cores || '—' }} {{ t('cores') }}</div>
            <div class="perf-bar-wrap"><div class="perf-bar cpu-bar" :style="{ width: Math.min(stats.cpu?.usage_percent || 0, 100) + '%' }"></div></div>
          </div>
          <div class="perf-stat">
            <div class="perf-stat-header"><i class="material-icons perf-stat-icon">storage</i><span class="perf-stat-label">{{ t('Disk Usage') }}</span></div>
            <div class="perf-stat-value">{{ stats.disk?.total_human || '—' }}</div>
            <div class="perf-stat-sub">{{ stats.disk?.usage_percent || '0' }}% {{ t('disk activity') }}</div>
            <div class="perf-bar-wrap"><div class="perf-bar disk-bar" :style="{ width: Math.min(stats.disk?.usage_percent || 0, 100) + '%' }"></div></div>
            <div class="perf-stat-breakdown">
              <span>{{ t('Read:') }} {{ stats.disk?.read_human || '—' }}</span>
              <span>{{ t('Write:') }} {{ stats.disk?.write_human || '—' }}</span>
              <span>{{ t('Total:') }} {{ stats.disk?.total_human || '—' }}</span>
            </div>
          </div>
          <div class="perf-stat">
            <div class="perf-stat-header">
              <i class="material-icons perf-stat-icon">cached</i>
              <span class="perf-stat-label">{{ t('Cache') }}</span>
              <div class="perf-stat-actions">
                <button class="perf-mini-btn" @click="openCacheFolder" :title="t('Open cache folder')">
                  <i class="material-icons">folder_open</i>
                </button>
                <button class="perf-mini-btn" @click="cacheHelpVisible = !cacheHelpVisible" :title="t('What is cache?')">
                  <i class="material-icons">help_outline</i>
                </button>
              </div>
            </div>
            <div class="perf-stat-value">{{ stats.cache?.cache_dir_human || '0 B' }}</div>
            <div class="perf-stat-breakdown">
              <span>{{ t('Temp:') }} {{ stats.cache?.temp_dir_human || '0 B' }}</span>
            </div>
            <p v-if="cacheHelpVisible" class="cache-help">
              {{ t('Cache stores temporary yt-dlp / EJS media files and aria2 session data. Open the folder to inspect or clean it.') }}
            </p>
          </div>
          </div>

        <div class="section-divider"></div>
        <div class="field row-field" style="margin-top: 20px;">
          <label>{{ t('Hardware Acceleration') }}</label>
          <div class="toggle-row">
            <div class="toggle" :class="{ active: hardwareAccel }" @click="toggleHardwareAccel" role="switch" :aria-checked="hardwareAccel">
              <div class="toggle-track"><div class="toggle-thumb"></div></div>
            </div>
          </div>
        </div>
        <p class="field-hint">{{ t('Disable to reduce GPU memory usage (30-50 MB savings). UI animations may feel sluggish. Restart required for changes to take effect.') }}</p>

        <div class="section-divider"></div>
        <div class="field row-field" style="margin-top: 20px;">
          <label>{{ t('Debug Logging') }}</label>
          <div class="toggle-row">
            <div class="toggle" :class="{ active: debugLogging }" @click="toggleDebugLogging" role="switch" :aria-checked="debugLogging">
              <div class="toggle-track"><div class="toggle-thumb"></div></div>
            </div>
            <button class="perf-mini-btn log-open-btn" @click="openLogFolder" :title="t('Open log folder')" v-if="debugLogging">
              <i class="material-icons">folder_open</i>
            </button>
          </div>
        </div>
        <p class="field-hint">{{ t('When enabled, the app writes timestamped activity (downloads, polls, errors) to moonhunt.log in the app data folder.') }}</p>

        <div class="section-divider"></div>
        <div class="db-explorer" v-if="dbFiles.length" style="margin-top: 20px;">
          <div class="db-explorer-header">
            <div>
              <h2 class="section-title" style="margin-bottom: 4px;">{{ t('App Database') }}</h2>
              <p class="db-explorer-desc">{{ t('Settings, download history, categories, and schedules.') }}</p>
            </div>
            <div class="db-explorer-actions">
              <button class="perf-mini-btn" @click="openDbFolder" :title="t('Open DB folder')">
                <i class="material-icons">folder_open</i>
              </button>
            </div>
          </div>
          <div class="db-explorer-path">
            <i class="material-icons">folder</i>
            <span class="db-explorer-path-text" :title="dbFolder">{{ dbFolder }}</span>
          </div>
          <div class="db-file-list">
            <div v-for="file in dbFiles" :key="file.name" class="db-file-row">
              <i class="material-icons db-file-icon">description</i>
              <span class="db-file-name" :title="file.name">{{ file.name }}</span>
              <span class="db-file-size">{{ formatBytes(file.size) }}</span>
              <button class="db-file-open" :title="t('Open in folder')" :aria-label="t('Open in folder')" @click="openDbFolder">
                <i class="material-icons">open_in_new</i>
              </button>
            </div>
          </div>
          <div class="db-explorer-footer">
            <button class="perf-mini-btn" @click="refreshDbFiles" :title="t('Refresh')">
              <i class="material-icons">refresh</i>
            </button>
          </div>
        </div>
                  </section>

    </main>
  </div>
</template>
<style scoped>
.settings-layout {
  display: flex;
  min-height: 100%;
  color: var(--text);
}

.settings-sidebar {
  position: fixed;
  top: 50px;
  left: var(--sidebar-width, 70px);
  width: 240px;
  height: calc(100vh - 50px);
  background: var(--bg-card);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
  z-index: 10;
}

.sidebar-header {
  padding: 20px 16px 16px;
}

.sidebar-title {
  margin: 0;
  font-size: 16px;
  font-weight: 700;
  color: var(--text);
}

.sidebar-nav {
  flex: 1;
  overflow-y: auto;
  padding: 8px 12px;
}

.sidebar-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 10px 12px;
  border-radius: 8px;
  background: transparent;
  border: none;
  color: var(--text-dim);
  font-size: 14px;
  font-family: inherit;
  cursor: pointer;
  text-align: left;
  transition: all 0.15s;
  margin-bottom: 2px;
}

.sidebar-item:hover {
  background: var(--hover-bg);
  color: var(--text);
}

.sidebar-item.active {
  background: rgba(var(--accent-rgb), 0.1);
  color: var(--accent-strong);
  font-weight: 600;
}

.sidebar-item i {
  font-size: 20px;
  width: 22px;
  text-align: center;
  flex-shrink: 0;
}

.sidebar-footer {
  margin-top: auto;
  padding: 12px 16px 16px;
  border-top: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.sidebar-save-btn {
  width: 100%;
  padding: 10px;
  border-radius: 8px;
  background: var(--accent);
  color: #0a0e1a;
  font-weight: 700;
  font-size: 13px;
  border: none;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  transition: opacity 0.15s;
}

.sidebar-save-btn:hover {
  opacity: 0.9;
}

.sidebar-reset-btn {
  width: 100%;
  padding: 8px;
  border-radius: 8px;
  background: var(--hover-bg);
  color: var(--text-dim);
  border: 1px solid var(--border);
  font-size: 12px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  transition: all 0.15s;
}

.sidebar-reset-btn:hover {
  color: var(--text);
  border-color: var(--border-strong);
}

.settings-content {
  flex: 1;
  overflow-y: auto;
  padding: 24px 32px 32px;
  margin-left: 240px;
}

.settings-section {
  margin-bottom: 32px;
  padding-bottom: 24px;
  border-bottom: 1px solid var(--border);
}

.settings-section:last-child {
  border-bottom: none;
}

.section-title {
  margin: 0 0 4px;
  font-size: 18px;
  font-weight: 700;
  letter-spacing: -0.01em;
}

.section-divider {
  height: 1px;
  background: var(--border);
  margin: 20px 0;
}

.section-sub {
  margin: 0 0 20px;
  color: var(--text-dim);
  font-size: 13px;
}

.field {
  margin-bottom: 16px;
}

.field:last-child {
  margin-bottom: 0;
}

.field label {
  display: block;
  color: var(--text-muted);
  font-size: 12px;
  font-weight: 600;
  margin-bottom: 6px;
  text-transform: uppercase;
  letter-spacing: 0.03em;
}

.field-hint {
  display: block;
  margin-top: 6px;
  color: var(--text-dim);
  font-size: 11px;
}

.auth-token-label {
  display: flex;
  align-items: center;
  gap: 4px;
}

.lamp-btn {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  padding: 0;
  border: none;
  background: transparent;
  color: var(--text-dim);
  cursor: help;
  border-radius: 4px;
}

.lamp-btn:hover {
  color: var(--accent, var(--text-strong));
  background: var(--border, transparent);
}

.lamp-btn .material-icons {
  font-size: 16px;
}

.lamp-tooltip {
  position: absolute;
  bottom: calc(100% + 8px);
  left: 0;
  z-index: 120;
  width: max-content;
  max-width: 280px;
  padding: 8px 10px;
  border-radius: 8px;
  background: var(--bg-card, #1e1e32);
  border: 1px solid var(--border-strong, rgba(255, 255, 255, 0.12));
  box-shadow: var(--shadow-md, 0 4px 16px rgba(0, 0, 0, 0.35));
  color: var(--accent-strong);
  font-size: 12px;
  font-weight: 400;
  line-height: 1.45;
  text-transform: none;
  letter-spacing: normal;
  pointer-events: none;
  opacity: 0;
  visibility: hidden;
  transition: opacity 0.1s ease;
}

.lamp-btn:hover .lamp-tooltip {
  opacity: 1;
  visibility: visible;
}

.field-row {
  display: flex;
  gap: 12px;
}

.field-row .field {
  flex: 1;
}

.optional {
  font-weight: 400;
  color: var(--text-dim);
  font-size: 11px;
}

.row-field {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.global-speed-limit {
  margin-bottom: 2px;
}

.global-speed-limit .speed-toggle-row {
  margin-bottom: 0;
}

.speed-control-row {
  display: flex;
  align-items: stretch;
  gap: 8px;
  flex-wrap: wrap;
}

.speed-control-row .stepper {
  margin-top: 0;
}

.speed-unit-switch {
  display: flex;
  height: 38px;
  border: 1px solid var(--border-strong);
  border-radius: 8px;
  overflow: hidden;
}

.speed-unit-btn {
  min-width: 44px;
  border: none;
  border-right: 1px solid var(--border-strong);
  background: var(--hover-bg);
  color: var(--text-dim);
  font-size: 11px;
  font-weight: 700;
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
}

.speed-unit-btn:last-child {
  border-right: none;
}

.speed-unit-btn:hover {
  background: rgba(var(--accent-rgb), 0.1);
  color: var(--accent-strong);
}

.speed-unit-btn.active {
  background: rgba(var(--accent-rgb), 0.12);
  color: var(--accent-strong);
}

input, select {
  width: 100%;
  height: 38px;
  background: var(--bg-surface);
  border: 1px solid var(--border-strong);
  color: var(--text);
  padding: 0 12px;
  border-radius: 8px;
  font-size: 13px;
  box-sizing: border-box;
  transition: border-color 0.2s ease, box-shadow 0.2s ease;
}

input:focus, select:focus {
  outline: none;
  border-color: var(--accent-strong);
  background: rgba(var(--accent-rgb), 0.03);
}

input::placeholder {
  color: var(--text-dim);
}

select {
  cursor: pointer;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='%23888'%3E%3Cpath d='M7 10l5 5 5-5z'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 12px center;
  padding-right: 32px;
}

select option {
  background: var(--bg-surface);
  color: var(--text);
}

.input-row {
  display: flex;
  gap: 0;
  align-items: stretch;
}

.input-row.fused input {
  border-radius: 8px 0 0 8px;
  border-right: none;
}

.input-row.fused .inline-btn {
  border-radius: 0;
}

.input-row.fused .inline-btn:last-child {
  border-radius: 0 8px 8px 0;
}

.input-row.fused.triple .inline-btn {
  border-radius: 0;
}

.input-row.fused.triple .inline-btn:last-child {
  border-radius: 0 8px 8px 0;
}

.input-row.fused:focus-within input,
.input-row.fused:focus-within .inline-btn,
.input-row.fused:focus-within .dl-limit-unit {
  border-color: var(--accent-strong);
  background: rgba(var(--accent-rgb), 0.03);
}

.inline-btn {
  height: 38px;
  width: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--hover-bg);
  border: 1px solid var(--border-strong);
  color: var(--text-dim);
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
}

.inline-btn .material-icons {
  font-size: 16px;
}

.inline-btn:hover {
  background: rgba(var(--accent-rgb), 0.1);
  color: var(--accent-strong);
}

.inline-btn.alt {
  background: rgba(var(--accent-rgb), 0.1);
  color: var(--accent-strong);
  border-color: rgba(var(--accent-rgb), 0.15);
}

.inline-btn.alt:hover {
  background: rgba(var(--accent-rgb), 0.2);
}

.stepper {
  display: flex;
  width: fit-content;
  border-radius: 8px;
  overflow: hidden;
  border: 1px solid var(--border-strong);
}

.stepper button {
  width: 36px;
  height: 38px;
  border: none;
  background: var(--hover-bg);
  color: var(--text-dim);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.15s ease, color 0.15s ease;
}

.stepper button:hover {
  background: rgba(var(--accent-rgb), 0.1);
  color: var(--accent-strong);
}

.stepper button:disabled {
  opacity: 0.35;
  cursor: not-allowed;
  background: transparent;
  color: var(--text-dim);
}

.stepper button .material-icons {
  font-size: 18px;
}

.stepper input {
  width: 56px;
  height: 38px;
  border-radius: 0;
  border: none;
  border-left: 1px solid var(--border-strong);
  border-right: 1px solid var(--border-strong);
  text-align: center;
  font-weight: 600;
}

.stepper input:focus {
  box-shadow: none;
}

.retry-row {
  display: flex;
  gap: 16px;
  margin-bottom: 10px;
}

.retry-row .retry-field {
  flex: 1;
  min-width: 0;
}

.retry-row .retry-field label {
  display: block;
  margin-bottom: 4px;
  color: var(--muted-foreground);
  font-size: 12px;
}

.retry-row .stepper {
  width: fit-content;
}

.retry-row .stepper input {
  width: 50px;
  text-align: center;
}

.toggle-header-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 6px;
}

.toggle-header-row .speed-toggle-row {
  margin-bottom: 0;
}

.speed-toggle-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
}

.speed-toggle {
  width: 42px;
  height: 24px;
  border-radius: 12px;
  background: var(--hover-bg);
  border: 1px solid var(--border-strong);
  cursor: pointer;
  position: relative;
  transition: background 0.25s ease, border-color 0.25s ease;
}

.speed-toggle.on {
  background: rgba(var(--accent-rgb), 0.25);
  border-color: rgba(var(--accent-rgb), 0.4);
}

.speed-toggle-knob {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--text-dim);
  transition: transform 0.25s cubic-bezier(0.34, 1.4, 0.64, 1), background 0.25s ease;
}

.speed-toggle.on .speed-toggle-knob {
  transform: translateX(18px);
  background: var(--accent);
}

.speed-toggle-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}

.speed-toggle-hint {
  font-size: 12px;
  color: var(--text-dim);
  margin-left: auto;
}

.speed-stepper {
  margin-top: 4px;
}

.speed-slide-enter-active {
  transition: opacity 0.2s ease, transform 0.2s ease;
}
.speed-slide-leave-active {
  transition: opacity 0.15s ease, transform 0.15s ease;
}
.speed-slide-enter-from {
  opacity: 0;
  transform: translateY(-6px);
}
.speed-slide-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}

.toggle-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.toggle {
  width: 42px;
  height: 24px;
  border-radius: 12px;
  background: var(--hover-bg);
  border: 1px solid var(--border-strong);
  cursor: pointer;
  position: relative;
  transition: background 0.25s ease, border-color 0.25s ease;
}

.toggle.active {
  background: rgba(var(--accent-rgb), 0.25);
  border-color: rgba(var(--accent-rgb), 0.4);
}

.toggle-track {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 18px;
  height: 18px;
  border-radius: 50%;
  background: var(--text-dim);
  transition: transform 0.25s cubic-bezier(0.34, 1.4, 0.64, 1), background 0.25s ease;
}

.toggle.active .toggle-track {
  transform: translateX(18px);
  background: var(--accent);
}

.info-box {
  background: rgba(var(--accent-rgb), 0.05);
  border-left: 3px solid rgba(var(--accent-rgb), 0.3);
  border-radius: 0 8px 8px 0;
  padding: 12px 14px;
  margin-bottom: 14px;
}

.info-title {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--accent-strong);
  font-size: 12px;
  font-weight: 700;
  margin-bottom: 8px;
}

.info-title .material-icons {
  font-size: 15px;
}

.tutorial-buttons {
  display: flex;
  gap: 8px;
}

.btn-tutorial {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: rgba(var(--accent-rgb), 0.1);
  color: var(--accent-strong);
  border: 1px solid rgba(var(--accent-rgb), 0.25);
  padding: 6px 14px;
  border-radius: 999px;
  cursor: pointer;
  font-size: 12px;
  font-weight: 700;
  font-family: inherit;
  transition: background 0.15s ease;
}

.btn-tutorial:hover {
  background: rgba(var(--accent-rgb), 0.2);
}

.btn-tutorial-disabled,
.btn-tutorial-disabled:hover {
  opacity: 0.5;
  cursor: not-allowed;
  background: var(--hover-bg);
  color: var(--text-dim);
  border-color: var(--border);
}

.btn-tutorial .material-icons {
  font-size: 14px;
}

.info-steps {
  margin: 0;
  padding-left: 18px;
  color: var(--text-dim);
  font-size: 11px;
  line-height: 1.8;
}

.info-steps li {
  margin-bottom: 2px;
}

.info-steps li:last-child {
  margin-bottom: 0;
}

.btn-config {
  background: rgba(var(--accent-rgb), 0.08);
  color: var(--accent-strong);
  border: 1px solid rgba(var(--accent-rgb), 0.15);
  padding: 10px 16px;
  border-radius: 999px;
  cursor: pointer;
  font-weight: 700;
  font-size: 13px;
  display: inline-flex;
  align-items: center;
  gap: 5px;
  transition: transform 0.2s ease;
}

.btn-config:hover {
  background: rgba(var(--accent-rgb), 0.15);
  transform: translateY(-1px);
}

.btn-config .material-icons { font-size: 16px; }

.full-width { width: 100%; }

.tutorial-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 2000;
  backdrop-filter: blur(4px);
}

.tutorial-modal {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 20px;
  width: 420px;
  max-width: 90vw;
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.4);
}

.tutorial-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 14px;
}

.tutorial-header .material-icons {
  color: var(--accent-strong);
  font-size: 20px;
}

.tutorial-header h3 {
  margin: 0;
  font-size: 14px;
  font-weight: 700;
  flex: 1;
}

.tutorial-close {
  background: none;
  border: none;
  color: var(--text-dim);
  cursor: pointer;
  padding: 4px;
  border-radius: 6px;
  display: inline-flex;
}

.tutorial-close:hover { background: var(--hover-bg); }

.tutorial-close .material-icons { font-size: 18px; }

.tutorial-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
  flex: 1;
  min-height: 0;
}

.tutorial-hint {
  margin: 0;
  font-size: 11.5px;
  color: var(--text-dim);
  padding: 10px 12px;
  border-radius: 8px;
  background: rgba(var(--accent-rgb), 0.06);
  border: 1px solid rgba(var(--accent-rgb), 0.15);
}

.tutorial-gif {
  flex: 1 1 auto;
  min-height: 0;
  width: auto;
  max-width: 100%;
  aspect-ratio: 720 / 405;
  border-radius: 8px;
  border: 1px solid var(--border);
}

.tutorial-modal-wide {
  display: flex;
  flex-direction: column;
  width: clamp(360px, 80vw, 760px);
  max-height: 90vh;
  overflow: hidden;
}

.tutorial-footer {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}

.tutorial-pager {
  display: flex;
  gap: 8px;
}

input[type="number"]::-webkit-outer-spin-button,
input[type="number"]::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}

input[type="number"] {
  -moz-appearance: textfield;
}

.color-swatches {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.swatch {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: border-color 0.15s, transform 0.15s;
  position: relative;
}

.swatch:hover {
  transform: scale(1.1);
}

.swatch.active {
  border-color: var(--text);
  box-shadow: 0 0 0 3px var(--bg-primary);
}

.swatch .material-icons {
  font-size: 14px;
  color: #fff;
  text-shadow: 0 1px 3px rgba(0, 0, 0, 0.5);
}

.mode-toggle-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.mode-toggle {
  width: 48px;
  height: 26px;
  border-radius: 13px;
  background: var(--hover-bg);
  border: 1px solid var(--border-strong);
  cursor: pointer;
  position: relative;
  transition: background 0.25s ease, border-color 0.25s ease;
}

.mode-toggle.light {
  background: rgba(var(--accent-rgb), 0.25);
  border-color: rgba(var(--accent-rgb), 0.4);
}

.mode-toggle-knob {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--text-dim);
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform 0.25s cubic-bezier(0.34, 1.4, 0.64, 1), background 0.25s ease;
}

.mode-toggle.light .mode-toggle-knob {
  transform: translateX(22px);
  background: var(--accent);
}

.mode-toggle-knob .material-icons {
  font-size: 12px;
  color: #fff;
}

.mode-toggle-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

.perf-error {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 12px 16px;
  background: rgba(244, 67, 54, 0.08);
  border: 1px solid rgba(244, 67, 54, 0.2);
  border-radius: 10px;
  color: #ef9a9a;
  font-size: 13px;
}

.perf-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 12px;
}

.perf-stat {
  background: var(--hover-bg);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 14px 16px;
}

.perf-stat-wide {
  grid-column: 1 / -1;
}

.perf-stat-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.perf-stat-icon {
  font-size: 16px;
  color: var(--accent-strong);
}

.perf-stat-label {
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0;
  color: var(--text-muted);
}

.perf-stat-value {
  font-size: 22px;
  font-weight: 800;
  color: var(--text);
  letter-spacing: -0.02em;
  margin-bottom: 2px;
}

.perf-stat-sub {
  font-size: 12px;
  color: var(--text-dim);
  margin-bottom: 10px;
}

.perf-bar-wrap {
  height: 4px;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 2px;
  overflow: hidden;
}

.perf-bar {
  height: 100%;
  background: var(--accent);
  border-radius: 2px;
  transition: width 0.4s ease;
}

.perf-bar.cpu-bar {
  background: linear-gradient(90deg, var(--accent), rgba(var(--accent-rgb), 0.5));
}

.perf-bar.disk-bar {
  background: linear-gradient(90deg, #42a5f5, rgba(66, 165, 245, 0.45));
}

.perf-stat-actions {
  display: flex;
  gap: 4px;
  margin-left: auto;
}

.perf-mini-btn {
  width: 24px;
  height: 24px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg-surface);
  color: var(--text-dim);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: color 0.15s ease, border-color 0.15s ease, background 0.15s ease;
}

.log-open-btn {
  width: 28px;
  height: 28px;
  margin-left: 8px;
}

.perf-mini-btn:hover {
  color: var(--accent-strong);
  border-color: rgba(var(--accent-rgb), 0.35);
  background: rgba(var(--accent-rgb), 0.06);
}

.perf-mini-btn .material-icons {
  font-size: 14px;
}

.cache-help {
  margin: 10px 0 0;
  padding: 8px 10px;
  border: 1px solid rgba(var(--accent-rgb), 0.18);
  border-radius: 8px;
  background: rgba(var(--accent-rgb), 0.06);
  color: var(--text-muted);
  font-size: 11px;
  line-height: 1.45;
}

.perf-stat-breakdown {
  display: flex;
  gap: 16px;
  margin-top: 8px;
}

.perf-stat-breakdown span {
  font-size: 11px;
  color: var(--text-dim);
}

/* ── App Database explorer ── */
.db-explorer {
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-card);
  padding: 16px;
}
.db-explorer-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 12px;
  margin-bottom: 10px;
}
.db-explorer-desc {
  margin: 0;
  font-size: 11px;
  color: var(--text-dim);
}
.db-explorer-actions {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
}
.db-explorer-path {
  display: flex;
  align-items: center;
  gap: 7px;
  padding: 4px 0;
  margin-bottom: 6px;
}
.db-explorer-path > i {
  font-size: 15px;
  color: var(--text-dim);
  flex-shrink: 0;
}
.db-explorer-path-text {
  flex: 1;
  min-width: 0;
  font-family: monospace;
  font-size: 11px;
  color: var(--text-dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.db-file-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  max-height: 240px;
  overflow-y: auto;
}
.db-file-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 8px;
  border: 1px solid transparent;
  border-radius: 6px;
  transition: background 0.12s ease, border-color 0.12s ease;
}
.db-file-row:hover {
  background: var(--hover-bg);
  border-color: var(--border);
}
.db-file-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  font-size: 18px;
  line-height: 1;
  color: var(--text-dim);
  flex-shrink: 0;
}
.db-file-name {
  flex: 1;
  min-width: 0;
  font-family: monospace;
  font-size: 12px;
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.db-file-size {
  flex-shrink: 0;
  font-size: 11px;
  color: var(--text-dim);
}
.db-file-open {
  width: 24px;
  height: 24px;
  border: none;
  background: transparent;
  color: var(--text-dim);
  border-radius: 6px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  flex-shrink: 0;
  transition: all 0.12s ease;
}
.db-file-open:hover {
  color: var(--accent-strong);
  background: rgba(var(--accent-rgb), 0.12);
}
.db-file-open i { font-size: 14px; }
.db-explorer-footer {
  display: flex;
  justify-content: flex-end;
  margin-top: 8px;
}

.limit-usage-bar {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.limit-usage-track {
  height: 6px;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 3px;
  overflow: hidden;
}

.limit-usage-fill {
  height: 100%;
  background: var(--accent);
  border-radius: 3px;
  transition: width 0.4s ease;
}

.limit-usage-text {
  font-size: 12px;
  color: var(--text-dim);
  font-weight: 600;
}

.dl-limit-unit {
  height: 38px;
  width: 72px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--hover-bg);
  border: 1px solid var(--border-strong);
  border-left: none;
  color: var(--text-dim);
  cursor: pointer;
  transition: background 0.15s ease, color 0.15s ease;
  font-size: 13px;
  font-weight: 600;
  padding: 0 8px;
  border-radius: 0 8px 8px 0;
}

.dl-limit-unit:hover {
  background: rgba(var(--accent-rgb), 0.1);
  color: var(--accent-strong);
}

.dl-limit-unit:focus {
  outline: none;
  border-color: var(--accent-strong);
  background: rgba(var(--accent-rgb), 0.03);
}

.dl-limit-unit-custom {
  position: absolute;
  right: 10px;
  width: 48px;
}

.dl-limit-unit-custom :deep(.custom-dropdown-trigger) {
  width: 100%;
  height: 26px;
  min-height: 26px;
  gap: 0;
  padding: 0 6px;
  background: transparent;
  border: none;
  border-radius: 7px;
  color: var(--text-muted);
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.4px;
  justify-content: center;
}

.dl-limit-unit-custom :deep(.custom-dropdown-trigger:hover) {
  background: rgba(var(--accent-rgb), 0.1);
  color: var(--accent-strong);
}

.dl-limit-unit-custom :deep(.custom-dropdown-trigger:focus) {
  background: rgba(var(--accent-rgb), 0.12);
  color: var(--accent-strong);
  border-color: transparent;
}

.dl-limit-unit-custom :deep(.custom-dropdown-value) {
  flex: 0 0 auto;
  text-align: center;
}

.dl-limit-unit-custom :deep(.custom-dropdown-arrow) {
  display: none;
}

.dl-limit-unit-custom :deep(.custom-dropdown-list) {
  left: auto;
  right: 0;
  width: 100%;
}

.dl-limit-unit-custom :deep(.custom-dropdown-item) {
  text-align: center;
  padding-left: 0;
  padding-right: 0;
}

.input-row.fused .dl-limit-unit {
  border-left: none;
}

.latency-list {
  margin-top: 10px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.latency-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 10px;
  border-radius: 6px;
  background: var(--hover-bg);
  font-size: 12px;
}

.latency-site {
  color: var(--text-dim);
}

.latency-value {
  font-weight: 600;
  color: var(--text-muted);
}

.latency-value.bad {
  color: var(--status-error);
}

/* ── Proxy Checker ── */
.proxy-checker { margin-top: 16px; display: flex; flex-direction: column; gap: 8px; }
.btn-check {
  display: inline-flex; align-items: center; gap: 6px; padding: 8px 16px;
  border-radius: 8px; border: 1px solid var(--border-strong); background: var(--bg-card);
  color: var(--text); font-size: 13px; font-weight: 600; cursor: pointer;
  transition: all 0.2s ease; width: fit-content;
}
.btn-check:hover:not(:disabled) { border-color: var(--accent-strong); color: var(--accent-strong); }
.btn-check:disabled { opacity: 0.5; cursor: not-allowed; }
.btn-check i { font-size: 18px; }
.proxy-test-result {
  display: flex; align-items: center; gap: 6px; padding: 8px 12px;
  border-radius: 8px; font-size: 12px; font-weight: 500;
}
.proxy-test-result.ok { background: rgba(129,199,132,0.08); color: var(--status-complete); }
.proxy-test-result.fail { background: rgba(239,83,80,0.08); color: var(--status-error); }
.proxy-test-result i { font-size: 16px; }

.btn-detect {
  display: inline-flex; align-items: center; gap: 4px;
  padding: 5px 10px; border-radius: 6px; border: 1px solid var(--border);
  background: var(--card); color: var(--text-dim); font-size: 11px;
  font-weight: 600; font-family: inherit; cursor: pointer;
  transition: all 0.15s; white-space: nowrap;
}
.btn-detect:hover:not(:disabled) { color: var(--accent-strong); border-color: rgba(var(--accent-rgb), 0.3); }
.btn-detect:disabled { opacity: 0.5; cursor: not-allowed; }
.detect-result {
  display: flex; align-items: center; gap: 6px; padding: 8px 12px;
  border-radius: 6px; font-size: 12px; font-weight: 600; margin-top: 4px; margin-bottom: 12px;
}
.detect-result.ok { background: rgba(129,199,132,0.08); color: var(--status-complete); }
.detect-result.fail { background: rgba(142,158,178,0.08); color: var(--text-dim); }
.detect-result i { font-size: 16px; }
@keyframes spin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }

.proxy-disabled-hint {
  display: flex; align-items: center; gap: 8px; padding: 12px 14px;
  background: var(--hover-bg); border-radius: 8px; font-size: 12px;
  color: var(--text-dim); margin-top: 4px;
}
.proxy-disabled-hint i { font-size: 16px; color: var(--text-dim); }

@media (max-width: 700px) {
  .settings-layout {
    flex-direction: column;
  }
  .settings-sidebar {
    position: relative;
    top: 0;
    left: 0;
    width: 100%;
    height: auto;
    border-right: none;
    border-bottom: 1px solid var(--border);
    flex-direction: row;
    overflow-x: auto;
    z-index: auto;
  }
  .sidebar-header {
    display: none;
  }
  .sidebar-nav {
    display: flex;
    gap: 4px;
    padding: 8px;
    overflow-y: visible;
  }
  .sidebar-item {
    white-space: nowrap;
    padding: 8px 12px;
  }
  .sidebar-footer {
    flex-direction: row;
    padding: 8px;
    border-top: none;
    border-left: 1px solid var(--border);
    margin-top: 0;
  }
  .settings-content {
    padding: 16px;
    margin-left: 0;
  }
}

/* ─── Language Cards ─── */
.language-cards {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 10px;
}
.language-card {
  position: relative;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 14px;
  height: 56px;
  min-height: 56px;
  overflow: hidden;
  background: var(--bg-card);
  border: 2px solid var(--border);
  border-radius: 12px;
  color: var(--text);
  font: inherit;
  text-align: left;
  cursor: pointer;
  transition: all 0.15s;
}
.language-card:hover {
  border-color: rgba(var(--accent-rgb), 0.3);
  transform: translateY(-2px);
}
.language-card.active {
  border-color: var(--accent-strong);
}
.language-flag {
  flex: none;
  width: 28px;
  height: 21px;
  border-radius: 4px;
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
  overflow: hidden;
}
.language-name {
  flex: 1;
  min-width: 0;
  font-size: 13px;
  font-weight: 600;
  line-height: 1.25;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.language-check {
  position: absolute;
  top: 8px;
  right: 8px;
  font-size: 18px;
  color: var(--accent-strong);
}

/* ─── Theme Cards ─── */
.theme-cards {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}
.theme-card {
  flex: 1;
  min-width: 140px;
  max-width: 200px;
  background: var(--bg-card);
  border: 2px solid var(--border);
  border-radius: 12px;
  padding: 0;
  cursor: pointer;
  transition: all 0.15s;
  overflow: hidden;
  position: relative;
}
.theme-card:hover {
  border-color: rgba(var(--accent-rgb), 0.3);
  transform: translateY(-2px);
}
.theme-card.active {
  border-color: var(--accent-strong);
}
.theme-card-preview {
  height: 80px;
  border-radius: 10px 10px 0 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 12px;
}
.dark-preview {
  background: #0a0e1a;
}
.light-preview {
  background: #f0ebe4;
}
.linear-preview {
  background: #08090a;
}
.preview-bar {
  height: 8px;
  border-radius: 4px;
  background: #1e1e32;
  width: 40%;
}
.preview-bar.light {
  background: #fffcf8;
}
.preview-bar.linear {
  background: #191a1b;
}
.preview-body {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.preview-line {
  height: 6px;
  border-radius: 3px;
  background: #16162a;
  width: 100%;
}
.preview-line.light {
  background: #f7f3ed;
}
.preview-line.linear {
  background: #0f1011;
}
.preview-line.short {
  width: 60%;
}
.theme-card-info {
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.theme-card-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}
.theme-card-desc {
  font-size: 11px;
  color: var(--text-dim);
}
.theme-card-check {
  position: absolute;
  top: 8px;
  right: 8px;
  font-size: 20px;
  color: var(--accent-strong);
}

/* Extension input — matches the Media save-location field */
.nd-input-wrap {
  position: relative;
  display: flex;
  align-items: center;
}

input.nd-input {
  width: 100%;
  height: 36px;
  background: rgba(255, 255, 255, 0.025);
  border: 1px solid var(--border);
  color: var(--text);
  padding: 0 10px;
  border-radius: 8px;
  font-size: 12px;
  font-family: inherit;
  outline: none;
  transition: all 0.15s ease;
  box-sizing: border-box;
}

input.nd-input:hover {
  border-color: rgba(255, 255, 255, 0.1);
  background: rgba(255, 255, 255, 0.035);
}

input.nd-input:focus {
  border-color: var(--accent-strong);
  background: rgba(var(--accent-rgb), 0.03);
}

input.nd-input::placeholder {
  color: var(--text-dim);
  opacity: 0.55;
}

/* Browser autofill paints its own pale blue/white background over text inputs,
   which made the token field differ from the number-based port field. Neutralize
   it so every nd-input keeps the themed background. */
input.nd-input:-webkit-autofill,
input.nd-input:-webkit-autofill:hover,
input.nd-input:-webkit-autofill:focus {
  -webkit-box-shadow: 0 0 0 1000px rgba(255, 255, 255, 0.025) inset;
  -webkit-text-fill-color: var(--text);
  caret-color: var(--text);
  transition: background-color 9999s ease-in-out 0s;
}

.nd-input-action {
  position: absolute;
  right: 10px;
  width: 26px;
  height: 26px;
  border-radius: 7px;
  background: transparent;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.nd-input-action i {
  font-size: 18px;
}

.nd-input-action:hover {
  background: rgba(var(--accent-rgb), 0.1);
  color: var(--accent-strong);
}

input.nd-path-input {
  padding-right: 36px;
  cursor: default;
}

.nd-input-wrap--token input.nd-token-input {
  padding-left: 34px;
  padding-right: 72px;
}

.nd-hide-btn {
  right: 42px;
}

.nd-input-lead {
  position: absolute;
  left: 10px;
  z-index: 1;
  width: 26px;
  height: 26px;
  border-radius: 7px;
  background: transparent;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.nd-input-lead i {
  font-size: 18px;
}

.nd-input-lead:hover {
  background: rgba(var(--accent-rgb), 0.1);
  color: var(--accent-strong);
}

.nd-secret-wrap input.nd-input {
  padding-right: 74px;
}

.nd-secret-action-visible {
  right: 42px;
}

.nd-secret-action-copy {
  right: 10px;
}

</style>
<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount, onActivated, onDeactivated, inject } from 'vue'
// -- Settings State --
import { invoke } from '@tauri-apps/api/core'
import { setDebugLogBridge } from '../composables/useDebugLog'
import { open } from '@tauri-apps/plugin-dialog'
import { resolveThemeSelection, useTheme } from '../composables/useTheme'
import { useGroups } from '../composables/useGroups'
import { useDownloadLimit } from '../composables/useDownloadLimit'
import { useGlobalSpeedLimit } from '../composables/useGlobalSpeedLimit'
import LanguageFlag from '../components/LanguageFlag.vue'
import CustomDropdown from '../components/CustomDropdown.vue'
import { LOCALES, normalizeLocale, setLocale, useI18n, type LocaleCode } from '../i18n'
import type { Settings } from '../types/settings'

const notify = inject<(msg: string, type: 'success' | 'error' | 'info') => void>('notify', (_msg, _type) => console.log(_msg))

const downloadLocation = ref('')
const maxConcurrent = ref(5)
const maxRetries = ref(5)
const retryPerUri = ref(3)
const retryWait = ref(3)
const maxDownloadSpeed = ref(0)
const speedLimitUnit = ref<'KB' | 'MB' | 'GB'>('KB')
const speedLimitInputValue = ref(0)
const speedLimitUnitOptions = [
  { value: 'KB' as const, label: 'KB' },
  { value: 'MB' as const, label: 'MB' },
  { value: 'GB' as const, label: 'GB' },
]
const speedLimitMultipliers: Record<'KB' | 'MB' | 'GB', number> = {
  KB: 1,
  MB: 1000,
  GB: 1_000_000,
}
const speedLimitEnabled = ref(false)
const globalSpeedLimit = useGlobalSpeedLimit()

watch(() => globalSpeedLimit.globalSpeedLimitEnabled.value, (enabled) => {
  if (globalSpeedLimit.loaded.value) {
    speedLimitEnabled.value = enabled
  }
})
const proxyType = ref('none')
const proxyHost = ref('')
const proxyPort = ref(1080)
const proxyUser = ref('')
const proxyPass = ref('')
const proxyEnabled = ref(false)
const proxyTesting = ref(false)
const proxyTestResult = ref<{ ok: boolean; status: number; latencyMs: number; message: string; ip?: string } | null>(null)
const detectingProxy = ref(false)
const detectResult = ref<{ enabled: boolean; host: string; port: number; message: string } | null>(null)
const rpcPort = ref(6800)
const extensionPort = ref(6802)
const extensionAuthEnabled = ref(false)
const extensionAuthToken = ref('')
const authTokenVisible = ref(false)
const isSaving = ref(false)
const protocolRegistered = ref(false)


const verifySsl = ref(true)
const searchPreviewEnabled = ref(true)
const hardwareAccel = ref(true)
const debugLogging = ref(false)
const i18n = useI18n()
const { t } = i18n
const language = ref<LocaleCode>('en')

function onLanguageChange(code: string) {
  language.value = normalizeLocale(code)
  setLocale(language.value)
}

// Torrent settings
const torrentDownloadLocation = ref('')
const torrentMaxConnections = ref(64)
const torrentDhtEnabled = ref(true)
const torrentAutoStart = ref(true)
const torrentUploadLimitEnabled = ref(false)
const torrentUploadLimit = ref(100)
const torrentUploadLimitUnit = ref<'KB' | 'MB' | 'GB'>('KB')
const torrentUploadLimitInputValue = ref(100)

const activeCategory = ref('general')
const categories = [
  { key: 'general', label: 'General', icon: 'tune' },
  { key: 'url', label: 'URL Downloads', icon: 'link' },
  { key: 'torrent', label: 'Torrent', icon: 'bolt' },
  { key: 'network', label: 'Network', icon: 'wifi' },
  { key: 'extension', label: 'Extension', icon: 'extension' },

  { key: 'appearance', label: 'Appearance', icon: 'palette' },
  { key: 'performance', label: 'Performance', icon: 'monitoring' },
]

function handleEsc(e: KeyboardEvent) {
  if (e.key === 'Escape' && activeCategory.value !== 'general') {
    activeCategory.value = 'general'
  }
}

onMounted(() => {
  loadSettings()
  document.addEventListener('keydown', handleEsc)
})

onActivated(() => {
  if (activeCategory.value === 'performance') startLiveStats()
  // Settings stays mounted via keep-alive, so transient proxy alerts from a
  // previous visit would otherwise still be visible on return.
  detectResult.value = null
  proxyTestResult.value = null
})

onDeactivated(stopLiveStats)

// -- Download Limit State --
const dlLimit = useDownloadLimit()
const dlLimitEnabled = ref(false)       // local copy — only applied on Save
const dlLimitBytes = ref(0)             // local copy — only applied on Save
const dlLimitUsedBytes = ref(0)         // local copy — display only, never written to global
const dlLimitUsagePercent = ref(0)
const dlLimitInputValue = ref(1)
const dlLimitUnit = ref<'KB' | 'MB' | 'GB'>('MB')
const dlLimitUnitOptions = [
  { value: 'KB', label: 'KB' },
  { value: 'MB', label: 'MB' },
  { value: 'GB', label: 'GB' },
]

// Keep local used bytes in sync with global (e.g. after reset from alert dialog)
watch(() => dlLimit.downloadLimitUsedBytes.value, (newVal) => {
  dlLimitUsedBytes.value = newVal
  updateDlLimitUsage()
})

function syncBytesFromInput() {
  const multipliers: Record<string, number> = { KB: 1024, MB: 1048576, GB: 1073741824 }
  dlLimitBytes.value = Math.round(dlLimitInputValue.value * multipliers[dlLimitUnit.value])
  updateDlLimitUsage()
}

function syncInputFromBytes(unit?: 'KB' | 'MB' | 'GB') {
  const bytes = dlLimitBytes.value
  if (bytes <= 0) { dlLimitInputValue.value = 1; dlLimitUnit.value = unit ?? 'MB'; return }
  const multipliers: Record<string, number> = { KB: 1024, MB: 1048576, GB: 1073741824 }
  if (unit && multipliers[unit]) {
    dlLimitUnit.value = unit
    dlLimitInputValue.value = bytes / multipliers[unit]
    return
  }
  if (bytes >= 1073741824 && bytes % 1073741824 === 0) {
    dlLimitUnit.value = 'GB'; dlLimitInputValue.value = bytes / 1073741824
  } else if (bytes >= 1048576 && bytes % 1048576 === 0) {
    dlLimitUnit.value = 'MB'; dlLimitInputValue.value = bytes / 1048576
  } else if (bytes >= 1024 && bytes % 1024 === 0) {
    dlLimitUnit.value = 'KB'; dlLimitInputValue.value = bytes / 1024
  } else {
    dlLimitUnit.value = 'MB'; dlLimitInputValue.value = Math.round(bytes / 1048576) || 1
  }
}

function toggleDlLimit() {
  dlLimitEnabled.value = !dlLimitEnabled.value
  // Immediately apply enable/disable to global (safety: stop tracking NOW)
  dlLimit.downloadLimitEnabled.value = dlLimitEnabled.value
  if (dlLimitEnabled.value && dlLimitBytes.value === 0) {
    dlLimitBytes.value = 1048576
    dlLimitInputValue.value = 1
    dlLimitUnit.value = 'MB'
  }
  // Reset counter when re-enabling — fresh start
  if (dlLimitEnabled.value) {
    dlLimitUsedBytes.value = 0
    dlLimit.downloadLimitUsedBytes.value = 0
  }
  updateDlLimitUsage()
}

function formatLimitDisplay(bytes: number): string {
  if (bytes <= 0) return '0 B'
  if (bytes >= 1073741824) {
    const gb = bytes / 1073741824
    return gb === Math.floor(gb) ? `${gb} GB` : `${gb.toFixed(2)} GB`
  }
  const mb = bytes / 1048576
  return mb === Math.floor(mb) ? `${mb} MB` : `${mb.toFixed(0)} MB`
}

function updateDlLimitUsage() {
  if (dlLimitBytes.value > 0) {
    dlLimitUsagePercent.value = Math.min(100, (dlLimitUsedBytes.value / dlLimitBytes.value) * 100)
  } else {
    dlLimitUsagePercent.value = 0
  }
}

// -- Performance Stats --
const stats = ref<Record<string, any>>({})
const statsLoading = ref(false)
const statsError = ref('')
const cacheHelpVisible = ref(false)
const dbFolder = ref('')
const dbFiles = ref<{ name: string; size: number; isDb: boolean }[]>([])
let statsTimer: number | null = null

async function refreshStats() {
  if (statsLoading.value) return
  statsLoading.value = true
  statsError.value = ''
  try {
    const result = await invoke<Record<string, any>>('get_system_stats', {})
    stats.value = result
  } catch (e: unknown) {
    statsError.value = 'Failed to load stats: ' + (e instanceof Error ? e.message : String(e))
  } finally {
    statsLoading.value = false
  }
}

function startLiveStats() {
  stopLiveStats()
  void refreshStats()
  statsTimer = window.setInterval(() => void refreshStats(), 1500)
}

function stopLiveStats() {
  if (statsTimer !== null) {
    window.clearInterval(statsTimer)
    statsTimer = null
  }
}

async function openCacheFolder() {
  try {
    await invoke('open_cache_folder')
  } catch (e: unknown) {
    notify('Failed to open cache folder: ' + (e instanceof Error ? e.message : String(e)), 'error')
  }
}

async function openDbFolder() {
  try {
    await invoke('open_app_db_folder')
  } catch (e: unknown) {
    notify('Failed to open DB folder: ' + (e instanceof Error ? e.message : String(e)), 'error')
  }
}

function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return '—'
  const units = ['B', 'KB', 'MB', 'GB']
  let size = bytes
  let i = 0
  while (size >= 1024 && i < units.length - 1) { size /= 1024; i++ }
  return `${i > 0 ? size.toFixed(1) : Math.round(size)} ${units[i]}`
}

async function refreshDbFiles() {
  try {
    const result = await invoke<{ folder: string; files: { name: string; size: number; is_db: boolean }[] }>('list_app_db_folder')
    dbFolder.value = result.folder
    dbFiles.value = (result.files || []).map((f) => ({ name: f.name, size: f.size, isDb: f.is_db }))
  } catch (e: unknown) {
    notify('Failed to list DB folder: ' + (e instanceof Error ? e.message : String(e)), 'error')
  }
}

watch(activeCategory, (category) => {
  if (category === 'performance') {
    startLiveStats()
    void refreshDbFiles()
  } else stopLiveStats()
  // Transient proxy alerts belong to the network tab; clear them on tab switch.
  if (category !== 'network') {
    detectResult.value = null
    proxyTestResult.value = null
  }
})

onBeforeUnmount(stopLiveStats)

const themeApi = useTheme()
const localTheme = ref('default')
const localMode = ref<'dark' | 'light' | 'linear-dark'>('dark')
const { reapplyAllLimits } = useGroups()

function syncSpeedLimitInput() {
  const kb = Math.max(0, Number(maxDownloadSpeed.value) || 0)
  if (kb <= 0) {
    speedLimitUnit.value = 'KB'
    speedLimitInputValue.value = 1
    return
  }
  if (kb >= 1_000_000) {
    speedLimitUnit.value = 'GB'
    speedLimitInputValue.value = kb / 1_000_000
  } else if (kb >= 1000) {
    speedLimitUnit.value = 'MB'
    speedLimitInputValue.value = kb / 1000
  } else {
    speedLimitUnit.value = 'KB'
    speedLimitInputValue.value = kb
  }
}

function applySpeedInput() {
  const raw = Number(speedLimitInputValue.value)
  if (!Number.isFinite(raw) || raw <= 0) {
    speedLimitInputValue.value = maxDownloadSpeed.value > 0
      ? Number((maxDownloadSpeed.value / speedLimitMultipliers[speedLimitUnit.value]).toFixed(2))
      : 1
    return
  }
  maxDownloadSpeed.value = Math.max(1, Math.round(raw * speedLimitMultipliers[speedLimitUnit.value]))
}

function stepSpeedLimit(direction: number) {
  const step = speedLimitUnit.value === 'KB' ? 100 : 1
  const parsed = Number(speedLimitInputValue.value)
  const current = Number.isFinite(parsed) ? parsed : 0
  const next = Math.max(current + direction * step, 0)
  speedLimitInputValue.value = Number(next.toFixed(2))
  applySpeedInput()
}

function setSpeedLimitUnit(unit: 'KB' | 'MB' | 'GB') {
  if (speedLimitUnit.value === unit) return
  // Keep the current number as-is — the unit only changes how the input is
  // interpreted (KB/s or MB/s or GB/s), not the displayed value.
  speedLimitUnit.value = unit
  applySpeedInput()
}

function syncTorrentUploadInput() {
  const kb = Math.max(0, Number(torrentUploadLimit.value) || 0)
  if (kb <= 0) { torrentUploadLimitInputValue.value = 1; torrentUploadLimitUnit.value = 'KB'; return }
  if (kb >= 1_000_000) {
    torrentUploadLimitUnit.value = 'GB'
    torrentUploadLimitInputValue.value = kb / 1_000_000
  } else if (kb >= 1000) {
    torrentUploadLimitUnit.value = 'MB'
    torrentUploadLimitInputValue.value = kb / 1000
  } else {
    torrentUploadLimitUnit.value = 'KB'
    torrentUploadLimitInputValue.value = kb
  }
}

function applyTorrentUploadInput() {
  const raw = Number(torrentUploadLimitInputValue.value)
  if (!Number.isFinite(raw) || raw <= 0) {
    torrentUploadLimitInputValue.value = torrentUploadLimit.value > 0
      ? Number((torrentUploadLimit.value / speedLimitMultipliers[torrentUploadLimitUnit.value]).toFixed(2))
      : 1
    return
  }
  torrentUploadLimit.value = Math.max(1, Math.round(raw * speedLimitMultipliers[torrentUploadLimitUnit.value]))
}

function stepTorrentUploadLimit(direction: number) {
  const step = torrentUploadLimitUnit.value === 'KB' ? 100 : 1
  const parsed = Number(torrentUploadLimitInputValue.value)
  const current = Number.isFinite(parsed) ? parsed : 0
  const next = Math.max(current + direction * step, 0)
  torrentUploadLimitInputValue.value = Number(next.toFixed(2))
  applyTorrentUploadInput()
}

function setTorrentUploadLimitUnit(unit: 'KB' | 'MB' | 'GB') {
  if (torrentUploadLimitUnit.value === unit) return
  // Keep the current number as-is — the unit only changes how the input is
  // interpreted, not the displayed value.
  torrentUploadLimitUnit.value = unit
  applyTorrentUploadInput()
}

function toggleSpeedLimit() {
  speedLimitEnabled.value = !speedLimitEnabled.value
  // When turning on with no value set, give a sensible default
  if (speedLimitEnabled.value && maxDownloadSpeed.value === 0) {
    maxDownloadSpeed.value = 200
    speedLimitInputValue.value = 200
    speedLimitUnit.value = 'KB'
  } else {
    syncSpeedLimitInput()
  }
  // When turning off, DON'T zero maxDownloadSpeed — preserve user's value
}

async function autoDetectProxy() {
  if (detectingProxy.value) return
  detectingProxy.value = true
  detectResult.value = null
  try {
    const result = await invoke<{ enabled: boolean; host: string; port: number; message: string }>('detect_system_proxy')
    detectResult.value = result
    if (result.enabled && result.host && result.port) {
      proxyHost.value = result.host
      proxyPort.value = result.port
      proxyType.value = 'http'
      proxyEnabled.value = true
    }
  } catch (e: any) {
    detectResult.value = { enabled: false, host: '', port: 0, message: `Detection failed: ${e}` }
  } finally {
    detectingProxy.value = false
  }
}

async function testProxyConnection() {
  if (proxyTesting.value) return
  proxyTesting.value = true
  proxyTestResult.value = null
  try {
    const result = await invoke<{ ok: boolean; status: number; latencyMs: number; message: string; ip?: string }>('test_proxy', {
      proxyType: proxyType.value,
      proxyHost: proxyHost.value,
      proxyPort: proxyPort.value,
      proxyUser: proxyUser.value || null,
      proxyPass: proxyPass.value || null,
    })
    proxyTestResult.value = result
    notify(result.ok ? `${t('Proxy connected')} (${result.latencyMs}ms)` : result.message, result.ok ? 'success' : 'error')
  } catch (e) {
    proxyTestResult.value = { ok: false, status: 0, latencyMs: 0, message: String(e) }
    notify(t('Proxy test failed') + ': ' + e, 'error')
  } finally {
    proxyTesting.value = false
  }
}

const DEFAULT_SETTINGS = {
  downloadLocation: '',  // set dynamically from backend
  maxConcurrent: 5,
  maxRetries: 5,
  retryPerUri: 3,
  retryWait: 3,
  maxDownloadSpeed: 0,
  globalSpeedLimitEnabled: false,
  verifySsl: true,
  searchPreviewEnabled: true,
  proxyType: 'none',
  proxyHost: '',
  proxyPort: 1080,
  proxyUser: '',
  proxyPass: '',
  rpcPort: 6800,
  extensionPort: 6802,
  extensionAuthEnabled: false,
  extensionAuthToken: '',
  openInBrowserProtocolEnabled: false,
  downloadLimitEnabled: false,
  downloadLimitBytes: 0,
  downloadLimitUsedBytes: 0,
  downloadLimitUnit: 'MB' as 'KB' | 'MB' | 'GB',
  disableHardwareAcceleration: false,
  debugLogging: false,
  torrentDownloadLocation: '',
  torrentMaxConnections: 64,
  torrentDhtEnabled: true,
  torrentAutoStart: true,
  torrentUploadLimitEnabled: false,
  torrentUploadLimit: 100,
}

// -- Load Settings from DB --
async function loadSettings() {
  try {
    // Get default download location from backend
    const defaultDir = await invoke<string>('get_default_download_dir')
    DEFAULT_SETTINGS.downloadLocation = defaultDir
    if (!downloadLocation.value) downloadLocation.value = defaultDir

    const saved = await invoke<any>('get_all_settings', {})
    if (saved) {
      downloadLocation.value = saved.downloadLocation || DEFAULT_SETTINGS.downloadLocation
      maxConcurrent.value = saved.maxConcurrent ?? DEFAULT_SETTINGS.maxConcurrent
      maxRetries.value = saved.maxRetries ?? DEFAULT_SETTINGS.maxRetries
      retryPerUri.value = saved.retryPerUri ?? DEFAULT_SETTINGS.retryPerUri
      retryWait.value = saved.retryWait ?? DEFAULT_SETTINGS.retryWait
      maxDownloadSpeed.value = saved.maxDownloadSpeed ?? DEFAULT_SETTINGS.maxDownloadSpeed
      speedLimitEnabled.value = saved.globalSpeedLimitEnabled ?? maxDownloadSpeed.value > 0
      globalSpeedLimit.globalSpeedLimitEnabled.value = speedLimitEnabled.value
      globalSpeedLimit.globalSpeedLimitKb.value = maxDownloadSpeed.value
      globalSpeedLimit.loaded.value = true
      syncSpeedLimitInput()
      verifySsl.value = saved.verifySsl ?? DEFAULT_SETTINGS.verifySsl
      searchPreviewEnabled.value = saved.searchPreviewEnabled ?? DEFAULT_SETTINGS.searchPreviewEnabled
      proxyType.value = saved.proxyType || DEFAULT_SETTINGS.proxyType
      proxyHost.value = saved.proxyHost || DEFAULT_SETTINGS.proxyHost
      proxyPort.value = saved.proxyPort ?? DEFAULT_SETTINGS.proxyPort
      proxyUser.value = saved.proxyUser || DEFAULT_SETTINGS.proxyUser
      proxyPass.value = saved.proxyPass || DEFAULT_SETTINGS.proxyPass
  proxyEnabled.value = saved.proxyType !== 'none' && saved.proxyType !== '' && saved.proxyType !== undefined
  // Ensure proxyType matches enabled state
  if (proxyEnabled.value && proxyType.value === 'none') proxyType.value = 'http'
      rpcPort.value = saved.rpcPort ?? DEFAULT_SETTINGS.rpcPort
      extensionPort.value = saved.extensionPort ?? DEFAULT_SETTINGS.extensionPort
      extensionAuthEnabled.value = saved.extensionAuthEnabled ?? DEFAULT_SETTINGS.extensionAuthEnabled
      extensionAuthToken.value = saved.extensionAuthToken ?? DEFAULT_SETTINGS.extensionAuthToken
      protocolRegistered.value = saved.openInBrowserProtocolEnabled ?? DEFAULT_SETTINGS.openInBrowserProtocolEnabled
      const selection = resolveThemeSelection(saved.theme || 'default', saved.themeMode || 'dark')
      localTheme.value = selection.themeName
      localMode.value = selection.mode
      language.value = normalizeLocale(saved.language || i18n.locale.value)
      hardwareAccel.value = saved.disableHardwareAcceleration === false
      debugLogging.value = saved.debugLogging ?? DEFAULT_SETTINGS.debugLogging
      dlLimitEnabled.value = saved.downloadLimitEnabled ?? DEFAULT_SETTINGS.downloadLimitEnabled
      dlLimitBytes.value = saved.downloadLimitBytes ?? DEFAULT_SETTINGS.downloadLimitBytes
      dlLimitUnit.value = saved.downloadLimitUnit === 'KB' || saved.downloadLimitUnit === 'MB' || saved.downloadLimitUnit === 'GB'
        ? saved.downloadLimitUnit
        : 'MB'
      dlLimitUsedBytes.value = dlLimit.downloadLimitUsedBytes.value
      dlLimit.downloadLimitEnabled.value = dlLimitEnabled.value
      updateDlLimitUsage()
      syncInputFromBytes(dlLimitUnit.value)
      // Torrent settings
      torrentDownloadLocation.value = saved.torrentDownloadLocation || ''
      torrentMaxConnections.value = saved.torrentMaxConnections ?? 64
      torrentDhtEnabled.value = saved.torrentDhtEnabled ?? true
      torrentAutoStart.value = saved.torrentAutoStart ?? true
      torrentUploadLimitEnabled.value = saved.torrentUploadLimitEnabled ?? false
      torrentUploadLimit.value = saved.torrentUploadLimit ?? 100
      syncTorrentUploadInput()
    }
  } catch (e) {
    console.error('Failed to load settings:', e)
  }
}

async function resetToDefaults() {
  if (confirm(t('Are you sure you want to reset to default settings?'))) {
    downloadLocation.value = DEFAULT_SETTINGS.downloadLocation
    maxConcurrent.value = DEFAULT_SETTINGS.maxConcurrent
    maxDownloadSpeed.value = DEFAULT_SETTINGS.maxDownloadSpeed
    speedLimitEnabled.value = false
    globalSpeedLimit.globalSpeedLimitEnabled.value = false
    globalSpeedLimit.globalSpeedLimitKb.value = 0
    syncSpeedLimitInput()
    proxyType.value = DEFAULT_SETTINGS.proxyType
    proxyHost.value = DEFAULT_SETTINGS.proxyHost
    proxyPort.value = DEFAULT_SETTINGS.proxyPort
    proxyUser.value = DEFAULT_SETTINGS.proxyUser
    proxyPass.value = DEFAULT_SETTINGS.proxyPass
    rpcPort.value = DEFAULT_SETTINGS.rpcPort
    extensionPort.value = DEFAULT_SETTINGS.extensionPort
    extensionAuthEnabled.value = DEFAULT_SETTINGS.extensionAuthEnabled
    extensionAuthToken.value = DEFAULT_SETTINGS.extensionAuthToken
    dlLimitEnabled.value = DEFAULT_SETTINGS.downloadLimitEnabled
    dlLimitBytes.value = DEFAULT_SETTINGS.downloadLimitBytes
    dlLimitUsedBytes.value = DEFAULT_SETTINGS.downloadLimitUsedBytes
    dlLimitUnit.value = DEFAULT_SETTINGS.downloadLimitUnit
    verifySsl.value = DEFAULT_SETTINGS.verifySsl
    searchPreviewEnabled.value = DEFAULT_SETTINGS.searchPreviewEnabled
    hardwareAccel.value = true
    language.value = 'en'
    setLocale('en')
    updateDlLimitUsage()
    syncInputFromBytes(DEFAULT_SETTINGS.downloadLimitUnit)
    // Torrent defaults
    torrentDownloadLocation.value = DEFAULT_SETTINGS.torrentDownloadLocation
    torrentMaxConnections.value = DEFAULT_SETTINGS.torrentMaxConnections
    torrentDhtEnabled.value = DEFAULT_SETTINGS.torrentDhtEnabled
    torrentAutoStart.value = DEFAULT_SETTINGS.torrentAutoStart
    torrentUploadLimitEnabled.value = DEFAULT_SETTINGS.torrentUploadLimitEnabled
    torrentUploadLimit.value = DEFAULT_SETTINGS.torrentUploadLimit
    syncTorrentUploadInput()

    // Save defaults to DB
    await saveSettings()
  }
}

// -- Save Settings to DB --
async function saveSettings() {
  if (isSaving.value) return
  isSaving.value = true

  // Read current saved settings first so unrelated fields aren't lost.
  let existingSettings: Partial<Settings> = {}
  try {
    existingSettings = await invoke<Partial<Settings>>('get_all_settings', {}) || {}
  } catch {}

  const settings = {
    ...existingSettings,
    theme: localTheme.value,
    themeMode: localMode.value,
    language: language.value,
    downloadLocation: downloadLocation.value,
    maxConcurrent: maxConcurrent.value,
    maxRetries: maxRetries.value,
    retryPerUri: retryPerUri.value,
    retryWait: retryWait.value,
    maxDownloadSpeed: maxDownloadSpeed.value,
    globalSpeedLimitEnabled: speedLimitEnabled.value,
    verifySsl: verifySsl.value,
    searchPreviewEnabled: searchPreviewEnabled.value,
    proxyType: proxyEnabled.value ? proxyType.value : 'none',
    proxyHost: proxyHost.value,
    proxyPort: proxyPort.value,
    proxyUser: proxyUser.value,
    proxyPass: proxyPass.value,
    rpcPort: rpcPort.value,
    extensionPort: extensionPort.value,
    extensionAuthEnabled: extensionAuthEnabled.value,
    extensionAuthToken: extensionAuthToken.value,
    openInBrowserProtocolEnabled: protocolRegistered.value,
    downloadLimitEnabled: dlLimitEnabled.value,
    downloadLimitBytes: dlLimitBytes.value,
    downloadLimitUsedBytes: dlLimitUsedBytes.value,
    downloadLimitUnit: dlLimitUnit.value,
    disableHardwareAcceleration: !hardwareAccel.value,
    debugLogging: debugLogging.value,
    torrentDownloadLocation: torrentDownloadLocation.value,
    torrentMaxConnections: torrentMaxConnections.value,
    torrentDhtEnabled: torrentDhtEnabled.value,
    torrentAutoStart: torrentAutoStart.value,
    torrentUploadLimitEnabled: torrentUploadLimitEnabled.value,
    torrentUploadLimit: torrentUploadLimit.value,
  }

  let saved = false
  try {
    // Save all settings to SQLite DB
    await invoke('save_all_settings', { settings })

    // Apply theme settings to the global state and DOM.
    themeApi.applyTheme(localTheme.value, localMode.value)

    // Apply download limit settings to the global tracking state
    dlLimit.downloadLimitEnabled.value = dlLimitEnabled.value
    dlLimit.downloadLimitBytes.value = dlLimitBytes.value
    dlLimit.downloadLimitUsedBytes.value = dlLimitUsedBytes.value

    globalSpeedLimit.globalSpeedLimitEnabled.value = speedLimitEnabled.value
    globalSpeedLimit.globalSpeedLimitKb.value = maxDownloadSpeed.value
    globalSpeedLimit.loaded.value = true

    saved = true
    notify(t('All settings saved successfully!'), 'success')
    // Let the rest of the app (e.g. titlebar search preview) pick up the change.
    window.dispatchEvent(new Event('moonhunt:settings-changed'))
  } catch (error) {
    notify(t('Error saving settings') + ': ' + error, 'error')
  } finally {
    isSaving.value = false
  }

  // Live aria2/extension work is slower than persistence. Run it after the
  // save completes so the Settings UI stays responsive, and only when the
  // values that affect the running daemon actually changed.
  if (saved) {
    void applyRuntimeSettings(existingSettings, settings as Partial<Settings>)
  }
}

async function applyRuntimeSettings(existingSettings: Partial<Settings>, settings: Partial<Settings>) {
  // Apply the moonhunt:// registry registration once the setting is committed.
  const protocolChanged = String(existingSettings.openInBrowserProtocolEnabled ?? '') !== String(settings.openInBrowserProtocolEnabled ?? '')
  if (protocolChanged) {
    try {
      if (settings.openInBrowserProtocolEnabled) {
        await invoke('set_default_protocol_client', { protocol: 'moonhunt' })
        notify(t('Open-app protocol enabled'), 'success')
      } else {
        await invoke('remove_as_default_protocol_client', { protocol: 'moonhunt' })
        notify(t('Open-app protocol disabled'), 'info')
      }
    } catch (e) {
      notify(t('Failed to change open-app protocol') + ': ' + e, 'error')
    }
  }

  const extensionKeys: (keyof Settings)[] = [
    'extensionPort', 'extensionAuthEnabled', 'extensionAuthToken', 'openInBrowserProtocolEnabled',
  ]
  const extensionChanged = extensionKeys.some(
    (k) => String(existingSettings[k] ?? '') !== String(settings[k] ?? '')
  )
  if (extensionChanged) {
    try {
      await invoke('set_extension_settings', {
        settings: {
          extensionPort: settings.extensionPort,
          extensionAuthEnabled: settings.extensionAuthEnabled,
          extensionAuthToken: settings.extensionAuthToken,
          openInBrowserProtocolEnabled: settings.openInBrowserProtocolEnabled,
        },
      })
    } catch (e) {
      console.error('Failed to update extension settings:', e)
    }
  }

  const aria2Affecting: (keyof Settings)[] = [
    'downloadLocation', 'maxConcurrent', 'maxDownloadSpeed',
    'globalSpeedLimitEnabled', 'verifySsl',
    'proxyType', 'proxyHost', 'proxyPort', 'proxyUser', 'proxyPass', 'rpcPort',
  ]
  const speedAffecting: (keyof Settings)[] = ['maxDownloadSpeed', 'globalSpeedLimitEnabled']
  const speedChanged = speedAffecting.some(
    (k) => String(existingSettings[k] ?? '') !== String(settings[k] ?? '')
  )
  const otherAria2Affecting = aria2Affecting.filter((k) => !speedAffecting.includes(k))
  const otherAria2Changed = otherAria2Affecting.some(
    (k) => String(existingSettings[k] ?? '') !== String(settings[k] ?? '')
  )
  if (!speedChanged && !otherAria2Changed) return

  try {
    if (speedChanged) {
      // Ensure the daemon is up (no-op when already running), then push the
      // new limit to new, active, queued and paused downloads immediately.
      await invoke('start_aria2_with_settings', { settings })
      await invoke('apply_global_speed_limit', { settings })
    }
    if (otherAria2Changed) {
      // Restart first — it spawns aria2 with --all-proxy from CLI and applies
      // credentials via RPC after the process is ready.
      await invoke('start_aria2_with_settings', { settings })
      // Apply proxy (and speed limits) to the running aria2 via RPC.
      await invoke('set_proxy', { settings })
    }
    await reapplyAllLimits()
  } catch (error) {
    console.error('Failed to apply runtime settings:', error)
    notify(t('Settings saved, but live download settings could not be applied.'), 'error')
  }
}

// Copy text to clipboard
function copyToClipboard(text: string) {
  navigator.clipboard.writeText(text).then(() => {
    notify(t('Copied to clipboard!'), 'success')
  }).catch((error) => {
    notify(t('Failed to copy') + ': ' + error.message, 'error')
  })
}

function generateAuthToken() {
  const bytes = new Uint8Array(8) // 8 bytes → 16 hex chars
  crypto.getRandomValues(bytes)
  extensionAuthToken.value = Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('')
}

const showTutorial = ref(false)
const tutorialBrowser = ref<'chrome' | 'firefox'>('chrome')
const tutorialPage = ref(1)
const isChrome = computed(() => tutorialBrowser.value === 'chrome')

function openTutorial(browser: 'chrome' | 'firefox') {
  tutorialBrowser.value = browser
  tutorialPage.value = 1
  showTutorial.value = true
}

function openTutorialRepo() {
  const url = 'https://github.com/2sulraw/MoonHunt-DownloadManager'
  invoke('open_external', { url }).catch(() => {
    window.open(url, '_blank', 'noopener')
  })
}

function openFirefoxAddonPage() {
  const url = 'https://addons.mozilla.org/en-US/firefox/addon/moonhunt-download-manager/'
  invoke('open_external', { url }).catch(() => {
    window.open(url, '_blank', 'noopener')
  })
}

// Toggle the moonhunt:// scheme preference. The registry change is applied on
// Save (see saveChanges), so this only updates the local draft state.
function toggleProtocol() {
  protocolRegistered.value = !protocolRegistered.value
}

// Select download location folder
async function selectDownloadLocation() {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath: downloadLocation.value || 'C:\\Downloads',
      title: t('Select Download Folder')
    })
    if (selected) {
      downloadLocation.value = selected as string
    }
  } catch (error) {
    console.error('Failed to select folder:', error)
    notify(t('Failed to select folder') + ': ' + error, 'error')
  }
}

// Select torrent download location folder
async function selectTorrentDownloadLocation() {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath: torrentDownloadLocation.value || downloadLocation.value || 'C:\\Downloads',
      title: t('Select Torrent Download Folder')
    })
    if (selected) {
      torrentDownloadLocation.value = selected as string
    }
  } catch (error) {
    console.error('Failed to select torrent folder:', error)
    notify(t('Failed to select torrent folder') + ': ' + error, 'error')
  }
}

async function toggleHardwareAccel() {
  hardwareAccel.value = !hardwareAccel.value
  try {
    const msg = await invoke('set_hardware_acceleration', { enabled: hardwareAccel.value })
    notify(String(msg), 'success')
  } catch (error) {
    notify(t('Failed to set hardware acceleration') + ': ' + error, 'error')
  }
}

async function toggleDebugLogging() {
  debugLogging.value = !debugLogging.value
  try {
    await invoke('set_debug_logging', { enabled: debugLogging.value })
    setDebugLogBridge(debugLogging.value)
    const label = debugLogging.value ? t('Debug logging enabled') : t('Debug logging disabled')
    notify(label, 'success')
  } catch (error) {
    notify(t('Failed to set debug logging') + ': ' + error, 'error')
  }
}

async function openLogFolder() {
  try {
    await invoke('open_log_folder')
  } catch (error) {
    notify(t('Failed to open log folder') + ': ' + error, 'error')
  }
}


</script>







