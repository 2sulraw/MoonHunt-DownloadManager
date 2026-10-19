<template>
  <div class="media-page">
    <!-- URL Input Card -->
    <section class="nd-card media-downloader-card">
      <header class="nd-card-header">
        <div class="nd-card-head-left">
          <div class="nd-avatar">
            <i class="material-icons">play_circle</i>
          </div>
          <div class="nd-titles">
            <h2 class="nd-title">{{ t('Media Downloader') }}</h2>
            <span class="nd-subtitle">{{ t('Paste a YouTube (or supported) URL to download.') }}</span>
          </div>
        </div>
      </header>

      <div class="nd-divider"></div>

      <div class="nd-card-body">
        <div class="nd-field">
          <label class="nd-label">
            <i class="material-icons nd-label-icon">link</i>
            {{ t('Video URL') }}
          </label>
          <div class="nd-input-wrap">
            <input class="nd-input" v-model="url" placeholder="https://www.youtube.com/watch?v=..." @keyup.enter="start" />
            <button v-if="url" class="nd-icon-btn nd-input-clear" @click="url = ''" :title="t('Clear')" :aria-label="t('Clear')">
              <i class="material-icons">close</i>
            </button>
          </div>
        </div>

        <div v-if="detected" class="media-detected">
          <i class="material-icons">check_circle</i>
          <span>{{ detected }}</span>
        </div>

        <div class="nd-field">
          <label class="nd-label">
            <i class="material-icons nd-label-icon">cookie</i>
            {{ t('Browser Cookies') }}
            <span class="nd-optional">{{ t('(for age-restricted content)') }}</span>
          </label>
          <div class="nd-tabs media-browser-tabs" role="tablist" :aria-label="t('Browser Cookies')">
            <span class="nd-tab-indicator" :class="{ right: selectedBrowser === 'chrome' }"></span>
            <button
              role="tab"
              class="nd-tab"
              :class="{ active: selectedBrowser === 'firefox' }"
              :aria-selected="selectedBrowser === 'firefox'"
              @click="selectedBrowser = 'firefox'"
            >
              <span class="browser-emoji">&#x1F43A;</span>
              <span>Firefox</span>
            </button>
            <button
              role="tab"
              class="nd-tab"
              :class="{ active: selectedBrowser === 'chrome' }"
              :aria-selected="selectedBrowser === 'chrome'"
              @click="selectedBrowser = 'chrome'"
            >
              <span class="browser-emoji">&#x1F310;</span>
              <span>Chrome</span>
            </button>
          </div>
          <p class="nd-field-hint">{{ t('Firefox is recommended. Select your browser to use its cookies for YouTube access.') }}</p>
        </div>

        <div class="nd-field">
          <label class="nd-label">
            <i class="material-icons nd-label-icon">folder</i>
            {{ t('Save Folder') }}
            <span class="nd-optional">{{ t('(optional)') }}</span>
          </label>
          <div class="nd-input-wrap">
            <input class="nd-input nd-path-input" v-model="path" placeholder="C:\Downloads" />
            <button class="nd-input-action" @click="browseFolder" :title="t('Browse')" :aria-label="t('Browse')">
              <i class="material-icons">folder_open</i>
            </button>
          </div>
        </div>
      </div>

      <footer class="nd-card-footer">
        <button class="nd-btn nd-btn-secondary media-footer-btn" @click="clear">
          <i class="material-icons">restart_alt</i>
          {{ t('Clear') }}
        </button>
        <button class="nd-btn nd-btn-primary media-footer-btn media-footer-primary" @click="startOrOpen" :disabled="!isStarting && !url.trim()">
          <i class="material-icons">{{ isStarting ? 'hourglass_top' : 'download' }}</i>
          {{ isStarting ? t('Scanning...') : t('Scan Formats') }}
        </button>
      </footer>

      <div v-if="resumeTaskVisible" class="media-resume">
        <div class="media-resume-text">
          <span v-if="downloadState === 'scanning'" class="media-spinner"></span>
          <span>{{ resumeTaskLabel }}</span>
        </div>
        <div class="media-resume-actions">
          <button class="nd-btn nd-btn-secondary media-resume-btn" @click="showMediaModal = true">
            <i class="material-icons">visibility</i>
            {{ t('View') }}
          </button>
          <button class="nd-btn nd-btn-danger media-resume-btn" @click="cancelMediaTask">
            <i class="material-icons">close</i>
            {{ t('Cancel') }}
          </button>
        </div>
      </div>
    </section>

    <!-- Binaries Card -->
    <section class="nd-card media-binaries-card tools-card">
      <header class="tools-card-header">
        <div class="tools-header-top">
          <div class="tools-header-icon">
            <i class="material-icons">build_circle</i>
          </div>
          <div class="nd-titles">
            <h2 class="nd-title">{{ t('Helper Binaries') }}</h2>
            <span class="nd-subtitle">{{ t('Required for media downloads to work.') }}</span>
          </div>
          <span class="tools-summary" :class="helperReady ? 'summary-ok' : 'summary-warn'">
            <button class="tools-summary-open" @click="openBinFolder" :title="t('Open folder')" :aria-label="t('Open folder')">
              <i class="material-icons">folder_open</i>
            </button>
            <i class="material-icons">{{ helperReady ? 'check_circle' : 'error_outline' }}</i>
            {{ helperReady ? t('Ready') : t('Needs setup') }}
          </span>
        </div>
      </header>

      <div class="tools-card-body">
        <div class="tool-item">
          <div class="tool-item-icon tool-icon-yt">
            <i class="material-icons">smart_display</i>
          </div>
          <div class="tool-item-main">
            <div class="tool-item-title">
              <span class="tool-item-name">yt-dlp</span>
              <span class="tool-status" :class="ytDlpExists ? 'status-ok-label' : 'status-idle-label'">
                <i class="material-icons">{{ ytDlpExists ? 'check' : 'download' }}</i>
                {{ ytDlpExists ? t('Installed') : t('Not installed') }}
              </span>
            </div>
            <p class="tool-item-desc">{{ t('Media download engine') }}</p>
            <div v-if="ytDlpExists && ytDlpPath" class="tool-location">
              <i class="material-icons">folder</i>
              <span class="tool-location-path" :title="ytDlpPath">{{ ytDlpPath }}</span>
              <button class="tool-location-open" :title="t('Open location')" @click="openYtDlpLocation">
                <i class="material-icons">open_in_new</i>
              </button>
            </div>
          </div>
          <div class="tool-actions">
            <template v-if="isInstallingYt">
              <div class="tool-install-state">
                <i class="material-icons spinning">downloading</i>
                <span>{{ installProgress > 0 ? installProgress + '%' : t('Installing...') }}</span>
              </div>
              <button class="tool-btn tool-btn-danger tool-btn-icon" :title="t('Cancel')" @click="cancelYtDlpInstall">
                <i class="material-icons">close</i>
              </button>
            </template>
            <template v-else-if="!ytDlpExists">
              <button class="tool-btn tool-btn-primary" @click="installYtDlp">
                <i class="material-icons">download</i>
                {{ t('Install') }}
              </button>
            </template>
            <template v-else>
              <button class="tool-btn tool-btn-secondary" @click="openYtDlpLocation">
                <i class="material-icons">folder_open</i>
                {{ t('Open location') }}
              </button>
              <button class="tool-btn tool-btn-danger" @click="uninstallYtDlp">
                <i class="material-icons">delete</i>
                {{ t('Remove') }}
              </button>
            </template>
          </div>
          <div v-if="isInstallingYt" class="tool-progress">
            <div class="tool-progress-fill" :style="{ width: installProgress + '%' }"></div>
          </div>
        </div>

        <div class="tool-item">
          <div class="tool-item-icon tool-icon-ejs">
            <i class="material-icons">code</i>
          </div>
          <div class="tool-item-main">
            <div class="tool-item-title">
              <span class="tool-item-name">EJS</span>
              <span class="tool-status" :class="ejsExists ? 'status-ok-label' : 'status-idle-label'">
                <i class="material-icons">{{ ejsExists ? 'check' : 'download' }}</i>
                {{ ejsExists ? t('Installed') : t('Not installed') }}
              </span>
            </div>
            <p class="tool-item-desc">{{ t('JavaScript runtime for YouTube signature solving') }}</p>
            <div v-if="ejsExists && ejsPath" class="tool-location">
              <i class="material-icons">folder</i>
              <span class="tool-location-path" :title="ejsPath">{{ ejsPath }}</span>
              <button class="tool-location-open" :title="t('Open location')" @click="openEjsLocation">
                <i class="material-icons">open_in_new</i>
              </button>
            </div>
          </div>
          <div class="tool-actions">
            <template v-if="isInstallingEjs">
              <div class="tool-install-state">
                <i class="material-icons spinning">downloading</i>
                <span>{{ t('Installing...') }}</span>
              </div>
              <button class="tool-btn tool-btn-danger tool-btn-icon" :title="t('Cancel')" @click="cancelEjsInstall">
                <i class="material-icons">close</i>
              </button>
            </template>
            <template v-else-if="!ejsExists">
              <button class="tool-btn tool-btn-primary" @click="installEjs">
                <i class="material-icons">download</i>
                {{ t('Install') }}
              </button>
            </template>
            <template v-else>
              <button class="tool-btn tool-btn-secondary" @click="openEjsLocation">
                <i class="material-icons">folder_open</i>
                {{ t('Open location') }}
              </button>
              <button class="tool-btn tool-btn-danger" @click="uninstallEjs">
                <i class="material-icons">delete</i>
                {{ t('Remove') }}
              </button>
            </template>
          </div>
        </div>
      </div>
    </section>

    <!-- JS Runtimes Card -->
    <section class="nd-card media-binaries-card tools-card">
      <header class="tools-card-header">
        <div class="tools-header-top">
          <div class="tools-header-icon">
            <i class="material-icons">terminal</i>
          </div>
          <div class="nd-titles">
            <h2 class="nd-title">{{ t('JS Runtimes') }}</h2>
            <span class="nd-subtitle">{{ t('Runtime used by yt-dlp to solve YouTube challenges.') }}</span>
          </div>
          <span class="tools-summary" :class="activeRuntimeName ? 'summary-ok' : 'summary-idle'">
            <button class="tools-summary-open" @click="openBinFolder" :title="t('Open folder')" :aria-label="t('Open folder')">
              <i class="material-icons">folder_open</i>
            </button>
            <i class="material-icons">{{ activeRuntimeName ? 'check_circle' : 'tune' }}</i>
            {{ activeRuntimeName || t('Not configured') }}
          </span>
        </div>
      </header>

      <div class="tools-card-body">
        <div class="tool-item system-tool-item">
          <div class="tool-item-icon system-tool-icon">
            <i class="material-icons">memory</i>
          </div>
          <div class="tool-item-main">
            <div class="tool-item-title">
              <span class="tool-item-name">{{ t('System Node.js') }}</span>
              <span class="tool-status" :class="systemNode ? 'status-ok-label' : 'status-idle-label'">
                <i class="material-icons">{{ systemNode ? 'check_circle' : 'help_outline' }}</i>
                {{ systemNode ? t('Detected') : t('Not detected') }}
              </span>
            </div>
            <p class="tool-item-desc">{{ systemNode ? systemNode.version : t('Not detected') }}</p>
            <div v-if="systemNode?.path" class="tool-location">
              <i class="material-icons">folder</i>
              <span class="tool-location-path" :title="systemNode.path">{{ systemNode.path }}</span>
            </div>
          </div>
          <div class="tool-actions">
            <span v-if="systemNode?.selected" class="tool-active-label">
              <i class="material-icons">check</i>
              {{ t('In use') }}
            </span>
            <button v-else-if="systemNode" class="tool-btn tool-btn-secondary" @click="selectJsRuntime('system-node')" :disabled="isSelectingRuntime || removingRuntime !== ''">
              <i class="material-icons">check</i>
              {{ t('Use') }}
            </button>
          </div>
        </div>

        <div class="tool-section-label">
          <span>{{ t('Runtime options') }}</span>
        </div>

        <div v-for="runtime in jsRuntimes" :key="runtime.id" class="tool-item runtime-item" :class="{ 'tool-item-active': runtime.selected && runtime.installed }">
          <div class="tool-item-icon" :class="runtime.installed ? 'runtime-icon-ok' : 'runtime-icon-idle'">
            <i class="material-icons">{{ runtimeIcon(runtime.id) }}</i>
          </div>
          <div class="tool-item-main">
            <div class="tool-item-title">
              <span class="tool-item-name">{{ runtime.name }}</span>
              <span class="tool-status" :class="runtime.installed ? 'status-ok-label' : 'status-idle-label'">
                <i class="material-icons">{{ runtime.selected && runtime.installed ? 'check_circle' : runtime.installed ? 'check' : 'download' }}</i>
                {{ runtime.selected && runtime.installed ? t('In use') : runtime.installed ? t('Installed') : t('Not installed') }}
              </span>
            </div>
            <p class="tool-item-desc">{{ runtime.size }}</p>
            <div v-if="runtime.installed && runtime.path" class="tool-location">
              <i class="material-icons">folder</i>
              <span class="tool-location-path" :title="runtime.path">{{ runtime.path }}</span>
            </div>
          </div>
          <div class="tool-actions">
            <template v-if="installingRuntime === runtime.id">
              <div class="tool-install-state">
                <i class="material-icons spinning">downloading</i>
                <span>{{ runtimeInstallPercent > 0 ? runtimeInstallPercent + '%' : t('Installing...') }}</span>
              </div>
              <button class="tool-btn tool-btn-danger tool-btn-icon" :title="t('Cancel')" @click="cancelJsRuntimeInstall(runtime.id)">
                <i class="material-icons">close</i>
              </button>
            </template>
            <template v-else>
              <span v-if="runtime.selected && runtime.installed" class="tool-active-label">
                <i class="material-icons">check</i>
                {{ t('In use') }}
              </span>
              <button v-else-if="runtime.installed" class="tool-btn tool-btn-secondary" @click="selectJsRuntime(runtime.id)" :disabled="isSelectingRuntime || removingRuntime !== ''">
                <i class="material-icons">check</i>
                {{ t('Use') }}
              </button>
              <button v-else class="tool-btn tool-btn-primary" @click="installJsRuntime(runtime.id)" :disabled="installingRuntime !== '' || removingRuntime !== ''">
                <i class="material-icons">download</i>
                {{ t('Download') }}
              </button>
              <button v-if="runtime.installed" class="tool-btn tool-btn-danger" @click="removeJsRuntime(runtime.id)" :disabled="removingRuntime !== '' || installingRuntime !== ''">
                <i class="material-icons" :class="{ spinning: removingRuntime === runtime.id }">delete</i>
                {{ removingRuntime === runtime.id ? t('Removing...') : t('Remove') }}
              </button>
            </template>
          </div>
          <div v-if="installingRuntime === runtime.id" class="tool-progress">
            <div class="tool-progress-fill" :style="{ width: runtimeInstallPercent + '%' }"></div>
          </div>
        </div>
      </div>
    </section>

    <!-- Media Merger Card -->
    <section class="nd-card media-binaries-card tools-card">
      <header class="tools-card-header">
        <div class="tools-header-top">
          <div class="tools-header-icon">
            <i class="material-icons">merge_type</i>
          </div>
          <div class="nd-titles">
            <h2 class="nd-title">{{ t('Media Merger') }}</h2>
            <span class="nd-subtitle">{{ t('Tool yt-dlp uses to combine video + audio into one file.') }}</span>
          </div>
          <span class="tools-summary" :class="activeMergerName ? 'summary-ok' : 'summary-idle'">
            <button class="tools-summary-open" @click="openBinFolder" :title="t('Open folder')" :aria-label="t('Open folder')">
              <i class="material-icons">folder_open</i>
            </button>
            <i class="material-icons">{{ activeMergerName ? 'check_circle' : 'tune' }}</i>
            {{ activeMergerName || t('Not configured') }}
          </span>
        </div>
      </header>

      <div class="tools-card-body">
        <div class="tool-item system-tool-item">
          <div class="tool-item-icon system-tool-icon">
            <i class="material-icons">memory</i>
          </div>
          <div class="tool-item-main">
            <div class="tool-item-title">
              <span class="tool-item-name">{{ t('System FFmpeg') }}</span>
              <span class="tool-status" :class="systemFfmpeg ? 'status-ok-label' : 'status-idle-label'">
                <i class="material-icons">{{ systemFfmpeg ? 'check_circle' : 'help_outline' }}</i>
                {{ systemFfmpeg ? t('Detected') : t('Not detected') }}
              </span>
            </div>
            <p class="tool-item-desc">{{ systemFfmpeg ? t('Use an already-installed FFmpeg') : t('Not detected') }}</p>
            <div v-if="systemFfmpeg?.path" class="tool-location">
              <i class="material-icons">folder</i>
              <span class="tool-location-path" :title="systemFfmpeg.path">{{ systemFfmpeg.path }}</span>
            </div>
          </div>
          <div class="tool-actions">
            <span v-if="systemFfmpeg?.selected" class="tool-active-label">
              <i class="material-icons">check</i>
              {{ t('In use') }}
            </span>
            <button v-else-if="systemFfmpeg" class="tool-btn tool-btn-secondary" @click="selectMerger('system-ffmpeg')" :disabled="isSelectingMerger || removingMerger !== ''">
              <i class="material-icons">check</i>
              {{ t('Use') }}
            </button>
          </div>
        </div>

        <div class="tool-section-label">
          <span>{{ t('Merger options') }}</span>
        </div>

        <div v-for="merger in mergers" :key="merger.id" class="tool-item runtime-item" :class="{ 'tool-item-active': merger.selected && merger.installed }">
          <div class="tool-item-icon" :class="merger.installed ? 'runtime-icon-ok' : 'runtime-icon-idle'">
            <i class="material-icons">{{ mergerIcon(merger.id) }}</i>
          </div>
          <div class="tool-item-main">
            <div class="tool-item-title">
              <span class="tool-item-name">{{ merger.name }}</span>
              <span class="tool-status" :class="merger.installed ? 'status-ok-label' : 'status-idle-label'">
                <i class="material-icons">{{ merger.selected && merger.installed ? 'check_circle' : merger.installed ? 'check' : 'download' }}</i>
                {{ merger.selected && merger.installed ? t('In use') : merger.installed ? t('Installed') : t('Not installed') }}
              </span>
            </div>
            <p class="tool-item-desc">{{ merger.size }}</p>
            <div v-if="merger.installed && merger.path" class="tool-location">
              <i class="material-icons">folder</i>
              <span class="tool-location-path" :title="merger.path">{{ merger.path }}</span>
            </div>
          </div>
          <div class="tool-actions">
            <template v-if="installingMerger === merger.id">
              <div class="tool-install-state">
                <i class="material-icons spinning">downloading</i>
                <span>{{ mergerInstallPercent > 0 ? mergerInstallPercent + '%' : t('Installing...') }}</span>
              </div>
              <button class="tool-btn tool-btn-danger tool-btn-icon" :title="t('Cancel')" @click="cancelMergerInstall(merger.id)">
                <i class="material-icons">close</i>
              </button>
            </template>
            <template v-else>
              <span v-if="merger.selected && merger.installed" class="tool-active-label">
                <i class="material-icons">check</i>
                {{ t('In use') }}
              </span>
              <button v-else-if="merger.installed" class="tool-btn tool-btn-secondary" @click="selectMerger(merger.id)" :disabled="isSelectingMerger || removingMerger !== ''">
                <i class="material-icons">check</i>
                {{ t('Use') }}
              </button>
              <button v-else class="tool-btn tool-btn-primary" @click="installMerger(merger.id)" :disabled="installingMerger !== '' || removingMerger !== ''">
                <i class="material-icons">download</i>
                {{ t('Download') }}
              </button>
              <button v-if="merger.installed" class="tool-btn tool-btn-danger" @click="removeMerger(merger.id)" :disabled="removingMerger !== '' || installingMerger !== ''">
                <i class="material-icons" :class="{ spinning: removingMerger === merger.id }">delete</i>
                {{ removingMerger === merger.id ? t('Removing...') : t('Remove') }}
              </button>
            </template>
          </div>
          <div v-if="installingMerger === merger.id" class="tool-progress">
            <div class="tool-progress-fill" :style="{ width: mergerInstallPercent + '%' }"></div>
          </div>
        </div>
      </div>
    </section>

    <!-- Download Modal -->
    <transition name="modal-fade">
      <div v-if="showMediaModal" class="nd-overlay" @mousedown.self.prevent="mediaOverlayPress()" @mouseup.self="mediaOverlayRelease() && minimizeMediaModal()" @mouseleave="mediaOverlayCancel()">
        <transition name="nd-slide">
          <div class="nd-modal media-nd-modal" @click.stop>
            <div class="nd-glow"></div>

            <header class="nd-header">
              <div class="nd-header-left">
                <div class="nd-avatar">
                  <i class="material-icons">play_circle</i>
                </div>
                <div class="nd-titles">
                  <h1 class="nd-title">{{ t('Media Download') }}</h1>
                  <span class="nd-subtitle">{{ t('Paste a YouTube (or supported) URL to download.') }}</span>
                </div>
              </div>
              <div class="nd-header-actions">
                <button class="nd-icon-btn nd-close" @click="minimizeMediaModal" :aria-label="t('Minimize')" :title="t('Minimize')">
                  <i class="material-icons">minimize</i>
                </button>
              </div>
            </header>

            <div class="nd-divider"></div>

            <div class="nd-body">
              <div v-if="dialedUrl" class="media-modal-url">{{ dialedUrl }}</div>

              <div v-if="downloadState !== 'error'" class="media-steps">
                <div class="media-step" :class="{ active: downloadState === 'scanning', done: ['selecting','downloading','running'].includes(downloadState) }">
                  <span class="media-step-dot"></span>
                  <span>{{ t('Scan') }}</span>
                </div>
                <div class="media-step-line" :class="{ done: ['selecting','downloading','running'].includes(downloadState) }"></div>
                <div class="media-step" :class="{ active: downloadState === 'selecting', done: ['downloading','running'].includes(downloadState) }">
                  <span class="media-step-dot"></span>
                  <span>{{ t('Select') }}</span>
                </div>
                <div class="media-step-line" :class="{ done: ['downloading','running'].includes(downloadState) }"></div>
                <div class="media-step" :class="{ active: ['downloading','running'].includes(downloadState) }">
                  <span class="media-step-dot"></span>
                  <span>{{ t('Download') }}</span>
                </div>
              </div>

              <div v-if="downloadState === 'scanning'" class="media-center">
                <div class="media-big-spinner"></div>
                <div class="media-center-text">{{ t('Scanning available qualities...') }}</div>
              </div>

              <div v-else-if="downloadState === 'selecting'" class="media-format-select">
                <div class="nd-section-header">
                  <h3 class="nd-section-title">{{ t('Available qualities') }}</h3>
                  <span v-if="!mergerAvailable" class="media-merger-hint" :title="t('Install or enable a media merger (FFmpeg) to unlock high-resolution formats.')">
                    <i class="material-icons">info</i> {{ t('Merger off — high-res hidden') }}
                  </span>
                </div>
                <div class="media-format-list">
                  <label v-for="f in visibleFormats" :key="f.format_id" class="media-format-item" :class="{ selected: selectedFormatId === f.format_id }">
                    <input type="radio" :value="f.format_id" v-model="selectedFormatId" class="media-format-radio" />
                    <span class="media-format-text">
                      {{ f.label }}
                      <small v-if="f.filesize" class="media-format-size">{{ formatBytes(f.filesize) }}</small>
                    </span>
                    <span v-if="f.requires_merger" class="media-format-tag tag-merger">{{ t('merger') }}</span>
                    <span v-else class="media-format-tag tag-direct">{{ t('direct') }}</span>
                  </label>
                </div>
              </div>

              <div v-else-if="downloadState === 'downloading'" class="media-center">
                <div class="media-big-spinner"></div>
                <div class="media-center-text">{{ t('Starting download...') }}</div>
              </div>

              <div v-else-if="downloadState === 'running'" class="media-center">
                <div class="media-check"><i class="material-icons">check</i></div>
                <div class="media-center-text">{{ t('Download started!') }}</div>
              </div>

              <div v-else-if="downloadState === 'error'" class="media-error-box">
                <div class="media-error-header">
                  <i class="material-icons">error</i>
                  <span>{{ t('Download failed') }}</span>
                </div>
                <pre class="media-error-pre">{{ downloadError }}</pre>
              </div>

              <div class="media-log-section">
                <div class="media-log-header">
                  <span class="media-log-title">{{ t('Log') }}</span>
                  <button v-if="ytLog.length" class="media-log-clear" @click="ytLog = []">{{ t('clear') }}</button>
                </div>
                <div ref="ytLogBox" class="media-log-box">
                  <pre v-if="ytLog.length"><span v-for="(line, i) in ytLog" :key="i" class="media-log-line">{{ line }}<br></span></pre>
                  <span v-else class="media-log-empty">{{ t('Waiting for output...') }}</span>
                </div>
              </div>
            </div>

            <footer class="nd-footer">
              <button class="nd-btn nd-btn-secondary" @click="minimizeMediaModal">
                <i class="material-icons">minimize</i>
                {{ t('Minimize') }}
              </button>
              <button v-if="['scanning','selecting','downloading','error'].includes(downloadState)" class="nd-btn nd-btn-danger" @click="cancelMediaTask">
                <i class="material-icons">close</i>
                {{ t('Cancel') }}
              </button>
              <button v-if="downloadState === 'selecting'" class="nd-btn nd-btn-primary" :disabled="!selectedFormatId" @click="downloadWithFormat">
                <i class="material-icons">download</i>
                {{ t('Download') }}
              </button>
              <button v-if="downloadState === 'error'" class="nd-btn nd-btn-primary" @click="start">
                <i class="material-icons">refresh</i>
                {{ t('Retry') }}
              </button>
            </footer>
          </div>
        </transition>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { ref, inject, computed, watch, onMounted, onActivated, onBeforeUnmount } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import { useOverlayClose } from '../composables/useOverlayClose'
import { useI18n } from '../i18n'

const { overlayPress: mediaOverlayPress, overlayCancel: mediaOverlayCancel, overlayRelease: mediaOverlayRelease } = useOverlayClose()
const { t } = useI18n()

const notify = inject<(msg: string, type: 'success' | 'error' | 'info') => void>('notify', (_m, _t) => console.log(_m))

const url = ref('')
const path = ref('')
const isStarting = ref(false)
const showMediaModal = ref(false)
type DownloadState = 'idle' | 'scanning' | 'selecting' | 'downloading' | 'running' | 'error'
const downloadState = ref<DownloadState>('idle')
const downloadError = ref('')
const dialedUrl = ref('')
const ytLog = ref<string[]>([])
const ytLogBox = ref<HTMLElement | null>(null)
const ytLogUnlisten = ref<(() => void) | null>(null)
const ytStartUnlisten = ref<(() => void) | null>(null)
const ytErrorUnlisten = ref<(() => void) | null>(null)
const ytCompleteUnlisten = ref<(() => void) | null>(null)
const ytInstallUnlisten = ref<(() => void) | null>(null)
const selectedBrowser = ref<'firefox' | 'chrome'>('firefox')
const ytStarted = ref(false)
let activeTaskId = 0
const activeScanId = ref<string | null>(null)

interface YtFormat {
  format_id: string; ext: string; resolution: string; vcodec: string; acodec: string
  has_video: boolean; has_audio: boolean; filesize: number | null; label: string
  requires_merger?: boolean
}

interface ScanFormatsResult {
  formats: YtFormat[]
  mergerAvailable: boolean
}

interface JsRuntimeInfo {
  id: string
  name: string
  installed: boolean
  selected: boolean
  path: string | null
  size: string
}

interface SystemNodeInfo {
  version: string
  path: string
  selected: boolean
}

interface JsRuntimesResult {
  runtimes: JsRuntimeInfo[]
  systemNode: SystemNodeInfo | null
}

interface MergerInfo {
  id: string
  name: string
  installed: boolean
  selected: boolean
  path: string | null
  size: string
}

interface SystemFfmpegInfo {
  path: string
  selected: boolean
}

interface MergersResult {
  mergers: MergerInfo[]
  systemFfmpeg: SystemFfmpegInfo | null
}

const scannedFormats = ref<YtFormat[]>([])
const scanMergerAvailable = ref(false)
const selectedFormatId = ref<string | null>(null)

function formatBytes(bytes: number | null): string {
  if (!bytes || bytes <= 0) return ''
  const units = ['B', 'KB', 'MB', 'GB']
  let size = bytes
  let i = 0
  while (size >= 1024 && i < units.length - 1) {
    size /= 1024
    i++
  }
  return `${size.toFixed(i > 0 ? 1 : 0)} ${units[i]}`
}

const isInstallingYt = ref(false)
const isInstallingEjs = ref(false)
const installProgress = ref(0)
const installTotal = ref(0)
const ytDlpExists = ref(false)
const ejsExists = ref(false)
const ytDlpPath = ref('')
const ejsPath = ref('')
const jsRuntimes = ref<JsRuntimeInfo[]>([])
const systemNode = ref<SystemNodeInfo | null>(null)
const installingRuntime = ref('')
const runtimeInstallPercent = ref(0)
const runtimeInstallUnlisten = ref<(() => void) | null>(null)
const isSelectingRuntime = ref(false)
const removingRuntime = ref('')

const mergers = ref<MergerInfo[]>([])
const systemFfmpeg = ref<SystemFfmpegInfo | null>(null)
const installingMerger = ref('')
const mergerInstallPercent = ref(0)
const mergerInstallUnlisten = ref<(() => void) | null>(null)
const isSelectingMerger = ref(false)
const removingMerger = ref('')

const activeMergerName = computed(() => {
  if (systemFfmpeg.value?.selected) return t('System FFmpeg')
  return mergers.value.find((merger) => merger.selected && merger.installed)?.name ?? ''
})

// A merger is usable if the backend resolved one (bundled ffmpeg is used even
// when not explicitly "Use"d), or a system ffmpeg is selected/detected.
const mergerAvailable = computed(() => {
  if (scanMergerAvailable.value) return true
  if (systemFfmpeg.value?.selected) return true
  return mergers.value.some((m) => m.installed)
})

// Formats shown to the user. Without a merger, video-only high-res formats
// (which need merging to have audio) are hidden; enabling a merger reveals
// them live.
const visibleFormats = computed(() =>
  scannedFormats.value.filter((f) => mergerAvailable.value || !f.requires_merger)
)

// Keep the selected format valid when the merger is toggled on/off.
watch(visibleFormats, (formats) => {
  if (formats.some((f) => f.format_id === selectedFormatId.value)) return
  const combined = formats.find((f) => f.has_video && f.has_audio)
  const audio = formats.find((f) => f.has_audio && !f.has_video)
  selectedFormatId.value = combined?.format_id ?? audio?.format_id ?? null
})

function mergerIcon(_mergerId: string): string {
  return 'movie_filter'
}

const helperReady = computed(() => ytDlpExists.value && ejsExists.value)
const activeRuntimeName = computed(() => {
  if (systemNode.value?.selected) return t('System Node.js')
  return jsRuntimes.value.find((runtime) => runtime.selected && runtime.installed)?.name ?? ''
})

function runtimeIcon(runtimeId: string): string {
  const icons: Record<string, string> = {
    quickjs: 'bolt',
    node: 'play_circle',
    bun: 'rocket_launch',
    deno: 'lens_blur',
  }
  return icons[runtimeId] || 'terminal'
}

const youtubeRe = /(?:youtube\.com\/watch\?v=|youtu\.be\/|youtube\.com\/shorts\/)/i
const detected = computed(() => {
  if (!url.value) return ''
  if (youtubeRe.test(url.value)) return t('YouTube')
  return t('Unknown/unsupported')
})

const resumeTaskVisible = computed(() => {
  if (showMediaModal.value) return false
  return ['scanning', 'selecting', 'downloading', 'error'].includes(downloadState.value)
})

const resumeTaskLabel = computed(() => {
  switch (downloadState.value) {
    case 'scanning': return t('Scanning in progress')
    case 'selecting': return t('Formats ready')
    case 'downloading': return t('Starting download')
    case 'error': return t('Media task needs attention')
    default: return ''
  }
})

onMounted(async () => {
  try {
    ytLogUnlisten.value = await listen<string>('yt-log', (e) => {
      const line = String(e.payload ?? '').replace(/\r?\n$/, '')
      if (line.length) ytLog.value.push(line)
      requestAnimationFrame(() => { if (ytLogBox.value) ytLogBox.value.scrollTop = ytLogBox.value.scrollHeight })
    })
  } catch (e) { console.warn(e) }

  try {
    ytStartUnlisten.value = await listen<{ gid: string; filename: string; url?: string }>('yt-dl-started', () => {
      // Ignore stale starts (e.g. the user cancelled while the backend kept
      // running) — only a pending download-start may claim the modal.
      if (downloadState.value !== 'downloading') return
      handleYtStarted()
    })
  } catch (e) { console.warn(e) }

  try {
    ytErrorUnlisten.value = await listen<{ gid: string; error: string }>('yt-dl-error', (e) => {
      downloadError.value = e.payload?.error || t('yt-dlp failed')
      downloadState.value = 'error'
      // Re-open the modal in error state so the user always sees the failure
      // detail and can retry, even if the window was closed when the download
      // was running in the background.
      showMediaModal.value = true
      notify(t('Media download failed'), 'error')
    })
  } catch (e) { console.warn(e) }

  try {
    ytCompleteUnlisten.value = await listen<{ gid: string; filename: string }>('yt-dl-complete', () => {
      downloadState.value = 'idle'
      showMediaModal.value = false
      notify(t('Download complete!'), 'success')
    })
  } catch (e) { console.warn(e) }
})

onActivated(() => {
  // One concurrent pass instead of four sequential IPC storms on every visit.
  void Promise.allSettled([refreshYtDlpState(), refreshEjsState(), loadJsRuntimes(), loadMergers()])
})

onBeforeUnmount(() => {
  try { ytLogUnlisten.value?.() } catch { /* */ }
  try { ytStartUnlisten.value?.() } catch { /* */ }
  try { ytErrorUnlisten.value?.() } catch { /* */ }
  try { ytCompleteUnlisten.value?.() } catch { /* */ }
  try { ytInstallUnlisten.value?.() } catch { /* */ }
  try { runtimeInstallUnlisten.value?.() } catch { /* */ }
})

async function browseFolder() {
  const selected = await open({ directory: true, multiple: false, defaultPath: path.value || 'C:\\Downloads', title: t('Select Save Folder') })
  if (selected) path.value = selected as string
}

async function start() {
  if (!url.value.trim()) { notify(t('Please enter a URL'), 'error'); return }
  if (!url.value.startsWith('http://') && !url.value.startsWith('https://')) { notify(t('Please enter a valid URL'), 'error'); return }

  const taskId = ++activeTaskId
  showMediaModal.value = true
  downloadState.value = 'scanning'
  downloadError.value = ''
  dialedUrl.value = url.value.trim()
  scannedFormats.value = []
  selectedFormatId.value = null
  ytLog.value = []
  ytStarted.value = false
  isStarting.value = true
  activeScanId.value = `media-scan-${Date.now()}-${Math.random().toString(36).slice(2)}`
  try {
      const res = await invoke<ScanFormatsResult>('scan_yt_formats', { url: url.value.trim(), browser: selectedBrowser.value, scanId: activeScanId.value })
      if (taskId !== activeTaskId) return
      activeScanId.value = null
      scanMergerAvailable.value = !!res.mergerAvailable
      scannedFormats.value = res.formats
      downloadState.value = 'selecting'
      // Auto-select the best visible (non-merge-required) combined format, or
      // an audio format, so there's always a sensible default.
      const combined = visibleFormats.value.find(f => f.has_video && f.has_audio)
      if (combined) {
        selectedFormatId.value = combined.format_id
      } else {
        const audio = visibleFormats.value.find(f => f.has_audio && !f.has_video)
        if (audio) selectedFormatId.value = audio.format_id
      }
      ytLog.value.push(
        `[scan] ${res.formats.length} format(s) found | merger ${res.mergerAvailable ? 'available' : 'missing'} | auto-selected: ${selectedFormatId.value ?? 'none'}`,
      )
    } catch (err: unknown) {
      if (taskId !== activeTaskId) return
      activeScanId.value = null
      downloadError.value = err instanceof Error ? err.message : String(err)
      downloadState.value = 'error'
    } finally { if (taskId === activeTaskId) isStarting.value = false }
  }

function startOrOpen() {
  if (isStarting.value) {
    showMediaModal.value = true
    return
  }
  void start()
}

async function downloadWithFormat() {
  if (!selectedFormatId.value) { notify(t('Please select a quality'), 'error'); return }
  const taskId = ++activeTaskId
  const sel = scannedFormats.value.find(f => f.format_id === selectedFormatId.value)
  // Audio-only selections (incl. the synthetic MP3 entry) → extract audio as
  // mp3. Video selections → mp4 (remux). This is what makes the chosen
  // format actually match the downloaded file instead of a forced mp4.
  const fmt = !sel || sel.has_video ? 'mp4' : 'mp3'
  downloadState.value = 'downloading'
  ytLog.value = []
  ytStarted.value = false
  // State the real selection up front — never a hardcoded "quality=best".
  ytLog.value.push(
    `[pick] ${sel?.label ?? selectedFormatId.value} | format_id=${selectedFormatId.value} | container=${fmt} | video=${sel?.has_video ?? '?'} audio=${sel?.has_audio ?? '?'}${sel?.filesize ? ` | ~${formatBytes(sel.filesize)}` : ''}`,
    `[pick] save to: ${path.value || '(default downloads folder)'} | cookies: ${selectedBrowser.value}`,
  )
  try {
    await invoke('add_yt_download', { url: dialedUrl.value, format: fmt, quality: undefined, path: path.value || undefined, browser: selectedBrowser.value, format_id: selectedFormatId.value, has_video: sel?.has_video, has_audio: sel?.has_audio })
    if (taskId !== activeTaskId) return
    handleYtStarted()
  } catch (err: unknown) {
    if (taskId !== activeTaskId) return
    downloadError.value = err instanceof Error ? err.message : String(err)
    downloadState.value = 'error'
  }
}

function handleYtStarted() {
  if (ytStarted.value) return
  ytStarted.value = true
  downloadState.value = 'running'
  showMediaModal.value = false
  window.dispatchEvent(new CustomEvent('yt-dlp-start', { detail: { url: dialedUrl.value } }))
  notify(t('Download started'), 'success')
}

function minimizeMediaModal() {
  showMediaModal.value = false
}

function cancelMediaTask() {
  activeTaskId++
  if (activeScanId.value) {
    const scanId = activeScanId.value
    activeScanId.value = null
    void invoke('cancel_yt_scan', { scanId }).catch((err: unknown) => console.warn('Failed to cancel scan:', err))
  }
  isStarting.value = false
  downloadState.value = 'idle'
  downloadError.value = ''
  showMediaModal.value = false
  scannedFormats.value = []
  selectedFormatId.value = null
  ytStarted.value = false
  notify(t('Media task cancelled'), 'info')
}

function clear() {
  url.value = ''; path.value = ''; downloadState.value = 'idle'; downloadError.value = ''
  showMediaModal.value = false; ytLog.value = []; scannedFormats.value = []; selectedFormatId.value = null; ytStarted.value = false; activeScanId.value = null
}

async function refreshYtDlpState() {
  try {
    ytDlpExists.value = await invoke('check_yt_dlp_exists') as boolean
    ytDlpPath.value = ytDlpExists.value ? await invoke('get_yt_dlp_path') as string : ''
  } catch (e) { console.error(e) }
}

async function refreshEjsState() {
  try {
    ejsExists.value = await invoke('check_ejs_exists') as boolean
    ejsPath.value = ejsExists.value ? await invoke('get_ejs_path') as string : ''
  } catch (e) { console.error(e) }
}

async function openYtDlpLocation() {
  try {
    await invoke('open_yt_dlp_location')
  } catch (err: unknown) {
    notify(t('Failed to open folder') + ': ' + String(err), 'error')
  }
}

async function openBinFolder() {
  try {
    await invoke('open_app_bin_folder')
  } catch (err: unknown) {
    notify(t('Failed to open folder') + ': ' + String(err), 'error')
  }
}

async function openEjsLocation() {
  try {
    await invoke('open_ejs_location')
  } catch (err: unknown) {
    notify(t('Failed to open folder') + ': ' + String(err), 'error')
  }
}

async function loadJsRuntimes() {
  try {
    const result = await invoke<JsRuntimesResult>('get_js_runtimes')
    jsRuntimes.value = result.runtimes
    systemNode.value = result.systemNode
  } catch (e) {
    console.error(e)
  }
}

async function installJsRuntime(runtimeId: string) {
  if (installingRuntime.value || removingRuntime.value) return
  installingRuntime.value = runtimeId
  runtimeInstallPercent.value = 0
  let unlisten: (() => void) | null = null
  try {
    try { runtimeInstallUnlisten.value?.() } catch { /* */ }
    unlisten = await listen<{ runtimeId: string; percent: number }>('js_runtime_install_progress', (e) => {
      if (e.payload?.runtimeId === runtimeId) {
        runtimeInstallPercent.value = Math.round(Number(e.payload.percent) || 0)
      }
    })
    runtimeInstallUnlisten.value = unlisten
    const res = await invoke('install_js_runtime', { runtimeId }) as string
    notify(res, 'success')
  } catch (err: unknown) {
    const msg = String(err)
    if (msg.toLowerCase().includes('cancelled')) {
      notify(t('Install cancelled'), 'info')
    } else {
      notify(t('Failed to install runtime') + ': ' + msg, 'error')
    }
  } finally {
    try { unlisten?.() } catch { /* */ }
    runtimeInstallUnlisten.value = null
    installingRuntime.value = ''
    runtimeInstallPercent.value = 0
    await loadJsRuntimes()
  }
}

async function cancelJsRuntimeInstall(runtimeId: string) {
  try {
    await invoke('cancel_js_runtime_install', { runtimeId })
  } catch (err: unknown) {
    console.warn('Failed to cancel runtime install:', err)
  }
}

async function selectJsRuntime(runtimeId: string) {
  if (isSelectingRuntime.value) return
  isSelectingRuntime.value = true
  try {
    const res = await invoke('select_js_runtime', { runtimeId }) as string
    notify(res, 'success')
    await loadJsRuntimes()
  } catch (err: unknown) {
    notify(t('Failed to select runtime') + ': ' + String(err), 'error')
  } finally {
    isSelectingRuntime.value = false
  }
}

async function removeJsRuntime(runtimeId: string) {
  if (removingRuntime.value || installingRuntime.value) return
  removingRuntime.value = runtimeId
  try {
    const res = await invoke('remove_js_runtime', { runtimeId }) as string
    notify(res, 'success')
    await loadJsRuntimes()
  } catch (err: unknown) {
    notify(t('Failed to remove runtime') + ': ' + String(err), 'error')
  } finally {
    removingRuntime.value = ''
  }
}

async function loadMergers() {
  try {
    const result = await invoke<MergersResult>('get_mergers')
    mergers.value = result.mergers
    systemFfmpeg.value = result.systemFfmpeg
  } catch (e) {
    console.error(e)
  }
}

async function installMerger(mergerId: string) {
  if (installingMerger.value || removingMerger.value) return
  installingMerger.value = mergerId
  mergerInstallPercent.value = 0
  let unlisten: (() => void) | null = null
  try {
    try { mergerInstallUnlisten.value?.() } catch { /* */ }
    unlisten = await listen<{ mergerId: string; percent: number }>('merger_install_progress', (e) => {
      if (e.payload?.mergerId === mergerId) {
        mergerInstallPercent.value = Math.round(Number(e.payload.percent) || 0)
      }
    })
    mergerInstallUnlisten.value = unlisten
    const res = await invoke('install_merger', { mergerId }) as string
    notify(res, 'success')
  } catch (err: unknown) {
    const msg = String(err)
    if (msg.toLowerCase().includes('cancelled')) {
      notify(t('Install cancelled'), 'info')
    } else {
      notify(t('Failed to install merger') + ': ' + msg, 'error')
    }
  } finally {
    try { unlisten?.() } catch { /* */ }
    mergerInstallUnlisten.value = null
    installingMerger.value = ''
    mergerInstallPercent.value = 0
    await loadMergers()
  }
}

async function cancelMergerInstall(mergerId: string) {
  try {
    await invoke('cancel_merger_install', { mergerId })
  } catch (err: unknown) {
    console.warn('Failed to cancel merger install:', err)
  }
}

async function selectMerger(mergerId: string) {
  if (isSelectingMerger.value) return
  isSelectingMerger.value = true
  try {
    const res = await invoke('select_merger', { mergerId }) as string
    notify(res, 'success')
    await loadMergers()
  } catch (err: unknown) {
    notify(t('Failed to select merger') + ': ' + String(err), 'error')
  } finally {
    isSelectingMerger.value = false
  }
}

async function removeMerger(mergerId: string) {
  if (removingMerger.value || installingMerger.value) return
  removingMerger.value = mergerId
  try {
    const res = await invoke('remove_merger', { mergerId }) as string
    notify(res, 'success')
    await loadMergers()
  } catch (err: unknown) {
    notify(t('Failed to remove merger') + ': ' + String(err), 'error')
  } finally {
    removingMerger.value = ''
  }
}

async function installYtDlp() {
  isInstallingYt.value = true; installProgress.value = 0
  try { ytInstallUnlisten.value?.() } catch { /* */ }
  const unlisten = await listen('yt_install_progress', (e) => {
    try { const p = e.payload as Record<string, unknown>; installProgress.value = Math.round((p.percent as number) || 0); installTotal.value = (p.total as number) || 0 } catch { /* */ }
  })
  try {
    const res = await invoke('install_yt_dlp') as string; notify(res, 'success')
    await refreshYtDlpState()
  } catch (err: unknown) {
    const msg = String(err)
    if (msg.toLowerCase().includes('cancelled')) {
      notify(t('Install cancelled'), 'info')
    } else {
      notify(t('Failed to install yt-dlp') + ': ' + msg, 'error')
    }
  }
  finally { try { unlisten() } catch { /* */ }; ytInstallUnlisten.value = null; isInstallingYt.value = false; installProgress.value = 0 }
}

async function cancelYtDlpInstall() {
  try {
    await invoke('cancel_yt_dlp_install')
  } catch (err: unknown) {
    console.warn('Failed to cancel yt-dlp install:', err)
  }
}

async function uninstallYtDlp() {
  try { const res = await invoke('uninstall_yt_dlp') as string; notify(res, 'success'); await refreshYtDlpState() } catch (err: unknown) { notify(t('Failed to uninstall yt-dlp') + ': ' + String(err), 'error') }
}

async function installEjs() {
  isInstallingEjs.value = true
  try {
    const res = await invoke('install_ejs') as string; notify(res, 'success'); await refreshEjsState()
  } catch (err: unknown) {
    const msg = String(err)
    if (msg.toLowerCase().includes('cancelled')) {
      notify(t('Install cancelled'), 'info')
    } else {
      notify(t('Failed to install EJS') + ': ' + msg, 'error')
    }
  }
  finally { isInstallingEjs.value = false }
}

async function cancelEjsInstall() {
  try {
    await invoke('cancel_ejs_install')
  } catch (err: unknown) {
    console.warn('Failed to cancel EJS install:', err)
  }
}

async function uninstallEjs() {
  try { const res = await invoke('uninstall_ejs') as string; notify(res, 'success'); await refreshEjsState() } catch (err: unknown) { notify(t('Failed to uninstall EJS') + ': ' + String(err), 'error') }
}
</script>

<style scoped>
.media-page {
  color: var(--text);
  display: flex;
  flex-direction: column;
  gap: 14px;
  width: min(720px, 100%);
  margin: 0 auto;
  padding: 4px 0 20px;
}




/* ============================================================
   Media page — New Download design language
   ============================================================ */
.nd-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 14px;
  box-shadow: var(--shadow-sm);
  overflow: hidden;
}

.nd-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px 10px;
  gap: 12px;
}

.nd-card-head-left {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  flex: 1;
}

.nd-card-header .nd-title { font-size: 15px; }

.nd-card-body {
  padding: 12px 16px 8px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.nd-card-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 16px 16px;
  border-top: 1px solid var(--border);
  flex-shrink: 0;
}

.nd-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.55);
  backdrop-filter: blur(4px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
  padding: 24px;
}

.nd-slide-enter-active { transition: opacity 0.15s ease; }
.nd-slide-leave-active { transition: opacity 0.1s ease; }
.nd-slide-enter-from, .nd-slide-leave-to { opacity: 0; }

.nd-glow {
  position: absolute;
  top: -60px;
  left: 50%;
  transform: translateX(-50%);
  width: 220px;
  height: 130px;
  background: radial-gradient(ellipse, rgba(var(--accent-rgb), 0.18), transparent 70%);
  pointer-events: none;
  z-index: 0;
  filter: blur(20px);
}

.nd-modal {
  position: relative;
  background: var(--bg-card);
  border: 1px solid var(--border-strong);
  border-radius: 14px;
  box-shadow: var(--shadow-lg);
  max-width: 480px;
  width: 100%;
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.nd-header {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 14px 16px 10px;
  border-bottom: 1px solid var(--border);
  gap: 12px;
}

.nd-header-left {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
  flex: 1;
}

.nd-avatar {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  background: linear-gradient(135deg, rgba(var(--accent-rgb), 0.12), rgba(var(--accent-rgb), 0.04));
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  border: 1px solid rgba(var(--accent-rgb), 0.12);
}

.nd-avatar i { font-size: 18px; color: var(--accent-strong); }
.nd-titles { min-width: 0; }
.nd-title {
  margin: 0;
  font-size: 14px;
  font-weight: 700;
  color: var(--text);
  letter-spacing: -0.01em;
  line-height: 1.2;
}
.nd-subtitle {
  font-size: 11px;
  color: var(--text-muted);
  font-weight: 400;
  margin-top: 2px;
  display: block;
}

.nd-header-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

.nd-icon-btn {
  position: relative;
  z-index: 1;
  background: none;
  border: none;
  color: var(--text-dim);
  width: 28px;
  height: 28px;
  border-radius: 7px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.12s ease;
  flex-shrink: 0;
  padding: 0;
}
.nd-icon-btn i { font-size: 16px; }
.nd-icon-btn:hover { background: rgba(255,255,255,0.06); color: var(--text); }
.nd-close:hover { background: rgba(239,83,80,0.1); color: var(--status-error); }

.nd-divider {
  height: 1px;
  background: linear-gradient(90deg, transparent, var(--border-strong), transparent);
  margin: 0 16px;
}

.nd-body {
  position: relative;
  z-index: 1;
  flex: 1;
  overflow-y: auto;
  padding: 12px 16px 8px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.nd-section {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.nd-section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}
.nd-section-title {
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-dim);
  margin: 0;
  opacity: 0.7;
}

.nd-field {
  display: flex;
  flex-direction: column;
  gap: 7px;
}
.nd-label {
  display: flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-muted);
  letter-spacing: 0.01em;
}
.nd-label-icon { font-size: 14px; color: var(--text-muted); opacity: 0.7; }
.nd-optional { font-weight: 400; color: var(--text-dim); font-size: 11px; }
.nd-field-hint {
  display: flex;
  align-items: flex-start;
  gap: 5px;
  font-size: 11px;
  color: var(--text-dim);
  margin: 2px 0 0;
  line-height: 1.45;
}

.nd-input-wrap {
  position: relative;
  display: flex;
  align-items: center;
}
.nd-input {
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
.nd-input:hover {
  border-color: rgba(255,255,255,0.1);
  background: rgba(255,255,255,0.035);
}
.nd-input:focus {
  border-color: var(--accent-strong);
  background: rgba(var(--accent-rgb), 0.03);
}
.nd-input::placeholder { color: var(--text-dim); opacity: 0.55; }
.nd-input-wrap:hover .nd-input-clear { opacity: 1; pointer-events: auto; }
.nd-input:focus ~ .nd-input-clear { opacity: 1; pointer-events: auto; }
.nd-input-clear {
  position: absolute;
  right: 8px;
  width: 24px;
  height: 24px;
  border-radius: 7px;
  background: transparent;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  pointer-events: none;
  transition: all 0.12s ease;
  padding: 0;
}
.nd-input-clear i { font-size: 13px; }
.nd-input-clear:hover { background: rgba(var(--accent-rgb), 0.15); color: var(--text); }
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
.nd-input-action i { font-size: 18px; }
.nd-input-action:hover { background: rgba(var(--accent-rgb), 0.1); color: var(--accent-strong); }
.nd-path-input { padding-right: 36px; cursor: default; }

/* Segmented switch — URL/Torrent style sliding indicator */
.nd-tabs {
  position: relative;
  display: flex;
  gap: 4px;
  background: var(--hover-bg);
  border-radius: 10px;
  padding: 4px;
}
.nd-tab {
  position: relative;
  z-index: 1;
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  height: 32px;
  border: none;
  border-radius: 7px;
  background: transparent;
  color: var(--text-dim);
  font-size: 12px;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.15s ease;
}
.nd-tab i { font-size: 16px; }
.nd-tab:hover:not(.active) { color: var(--text-muted); background: rgba(255,255,255,0.04); }
.nd-tab.active { color: var(--accent-strong); }
.nd-tab-indicator {
  position: absolute;
  top: 4px;
  left: 4px;
  width: calc(50% - 6px);
  height: 32px;
  background: var(--bg-card);
  border-radius: 7px;
  box-shadow: 0 1px 3px rgba(0,0,0,0.18), inset 0 0 0 1px var(--border);
  pointer-events: none;
  transition: transform 0.22s cubic-bezier(0.4, 0, 0.2, 1);
  z-index: 0;
}
.nd-tab-indicator.right { transform: translateX(calc(100% + 4px)); }
.media-browser-tabs { margin-top: 2px; }
.browser-emoji { font-size: 13px; line-height: 1; }

/* Card actions */
.media-footer-btn {
  flex: 0 0 auto;
}
.media-footer-primary { min-width: 150px; }

.media-resume {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin: 0 16px 16px;
  padding: 10px 12px;
  background: rgba(255,255,255,0.02);
  border: 1px solid var(--border);
  border-radius: 10px;
}
.media-resume-text {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-dim);
  font-size: 12px;
  font-weight: 600;
}
.media-resume-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}
.media-resume-btn {
  flex: 0 0 auto;
  height: 32px;
  padding: 7px 12px;
  border-radius: 8px;
  font-size: 11.5px;
}

.media-detected {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 10px;
  background: rgba(129, 199, 132, 0.08);
  border: 1px solid rgba(129, 199, 132, 0.18);
  border-radius: 100px;
  color: var(--status-complete);
  font-size: 11px;
  font-weight: 700;
  align-self: flex-start;
}
.media-detected i { font-size: 14px; }

/* Binaries */
.tools-card { position: relative; }
.tools-card-header {
  position: relative;
  overflow: hidden;
  padding: 16px 16px 12px;
  border-bottom: 1px solid var(--border);
}
.tools-card-header::before {
  content: '';
  position: absolute;
  top: -70px;
  right: -30px;
  width: 200px;
  height: 130px;
  background: radial-gradient(circle, rgba(var(--accent-rgb), 0.16), transparent 70%);
  pointer-events: none;
}
.tools-header-top {
  position: relative;
  z-index: 1;
  display: flex;
  align-items: center;
  gap: 12px;
}
.tools-header-icon {
  width: 38px;
  height: 38px;
  border-radius: 12px;
  background: linear-gradient(135deg, rgba(var(--accent-rgb), 0.18), rgba(var(--accent-rgb), 0.05));
  border: 1px solid rgba(var(--accent-rgb), 0.16);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  box-shadow: 0 6px 16px rgba(var(--accent-rgb), 0.1);
}
.tools-header-icon i { font-size: 20px; color: var(--accent-strong); }
.tools-summary {
  margin-inline-start: auto;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 5px 10px;
  border-radius: 100px;
  font-size: 11px;
  font-weight: 700;
  background: var(--hover-bg);
  color: var(--text-muted);
  border: 1px solid var(--border);
  flex-shrink: 0;
}
.tools-summary i { font-size: 14px; }
.tools-summary-open {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  margin-left: -4px;
  border: none;
  background: transparent;
  color: currentColor;
  opacity: 0.7;
  border-radius: 6px;
  cursor: pointer;
  padding: 0;
  transition: all 0.12s ease;
}
.tools-summary-open i { font-size: 14px; }
.tools-summary-open:hover {
  opacity: 1;
  color: var(--accent-strong);
  background: rgba(var(--accent-rgb), 0.12);
}
.summary-ok {
  color: var(--status-complete);
  background: rgba(129, 199, 132, 0.08);
  border-color: rgba(129, 199, 132, 0.18);
}
.summary-warn {
  color: #ffd180;
  background: rgba(255, 209, 128, 0.08);
  border-color: rgba(255, 209, 128, 0.16);
}
.summary-idle { color: var(--text-dim); }

.tools-card-body {
  padding: 12px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.tool-item {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: 14px;
  background: linear-gradient(135deg, rgba(255, 255, 255, 0.025), rgba(255, 255, 255, 0.008));
  flex-wrap: wrap;
  position: relative;
  transition: border-color 0.15s ease, background 0.15s ease;
}
.tool-item:hover {
  border-color: var(--border-strong);
  background: linear-gradient(135deg, rgba(255, 255, 255, 0.04), rgba(255, 255, 255, 0.012));
}
.tool-item-icon {
  width: 42px;
  height: 42px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  border: 1px solid var(--border-strong);
}
.tool-item-icon i { font-size: 20px; }
.tool-icon-yt {
  color: var(--accent-strong);
  background: rgba(var(--accent-rgb), 0.12);
  border-color: rgba(var(--accent-rgb), 0.18);
}
.tool-icon-ejs {
  color: #b39ddb;
  background: rgba(179, 157, 219, 0.1);
  border-color: rgba(179, 157, 219, 0.16);
}
.system-tool-icon {
  color: #9fb8ff;
  background: rgba(159, 184, 255, 0.1);
  border-color: rgba(159, 184, 255, 0.16);
}
.runtime-icon-ok {
  color: var(--status-complete);
  background: rgba(129, 199, 132, 0.1);
  border-color: rgba(129, 199, 132, 0.18);
}
.runtime-icon-idle {
  color: var(--accent-strong);
  background: rgba(var(--accent-rgb), 0.08);
  border-color: rgba(var(--accent-rgb), 0.14);
}
.tool-item-main {
  flex: 1 1 220px;
  min-width: 0;
}

.tool-item-title {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}
.tool-item-name {
  font-size: 13.5px;
  font-weight: 700;
  letter-spacing: -0.01em;
}
.tool-status {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 3px 8px;
  border-radius: 100px;
  font-size: 10.5px;
  font-weight: 700;
}
.tool-status i { font-size: 12px; }
.status-ok-label {
  color: var(--status-complete);
  background: rgba(129, 199, 132, 0.08);
  border: 1px solid rgba(129, 199, 132, 0.14);
}
.status-idle-label {
  color: var(--text-muted);
  background: var(--hover-bg);
  border: 1px solid var(--border);
}
.tool-item-desc {
  margin: 4px 0 0;
  font-size: 11px;
  color: var(--text-dim);
  line-height: 1.4;
}
.tool-location {
  display: flex;
  align-items: center;
  gap: 7px;
  margin-top: 8px;
  padding: 6px 8px;
  background: var(--hover-bg);
  border: 1px solid var(--border);
  border-radius: 8px;
  direction: ltr;
  min-width: 0;
}
.tool-location > i {
  font-size: 14px;
  color: var(--text-dim);
  flex-shrink: 0;
}
.tool-location-path {
  flex: 1;
  min-width: 0;
  font-family: monospace;
  font-size: 10px;
  color: var(--text-dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tool-location-open {
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
.tool-location-open:hover {
  color: var(--accent-strong);
  background: rgba(var(--accent-rgb), 0.12);
}
.tool-location-open i { font-size: 13px; }
.tool-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  flex-shrink: 0;
  flex-wrap: wrap;
}
.tool-install-state {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 0 2px;
  font-size: 11.5px;
  font-weight: 700;
  color: var(--accent-strong);
}
.tool-install-state i { font-size: 15px; }
.tool-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 32px;
  padding: 0 11px;
  border-radius: 9px;
  border: 1px solid transparent;
  font-family: inherit;
  font-size: 11px;
  font-weight: 700;
  cursor: pointer;
  transition: all 0.15s ease;
  flex: 0 0 auto;
}
.tool-btn i { font-size: 15px; }
.tool-btn:disabled { opacity: 0.45; cursor: not-allowed; }
.tool-btn-primary {
  background: var(--accent);
  color: #07121d;
  box-shadow: 0 4px 12px rgba(var(--accent-rgb), 0.18);
}
.tool-btn-primary:hover:not(:disabled) {
  filter: brightness(1.08);
  box-shadow: 0 5px 16px rgba(var(--accent-rgb), 0.24);
}
.tool-btn-secondary {
  background: var(--hover-bg);
  border-color: var(--border-strong);
  color: var(--text-muted);
}
.tool-btn-secondary:hover:not(:disabled) {
  color: var(--text);
  background: rgba(255, 255, 255, 0.08);
}
.tool-btn-danger {
  background: rgba(239, 83, 80, 0.1);
  border-color: rgba(239, 83, 80, 0.16);
  color: var(--status-error);
}
.tool-btn-danger:hover:not(:disabled) {
  background: rgba(239, 83, 80, 0.16);
}
.tool-btn-icon {
  width: 32px;
  padding: 0;
  justify-content: center;
}
.tool-active-label {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 5px 9px;
  border-radius: 100px;
  font-size: 11px;
  font-weight: 700;
  color: var(--status-complete);
  background: rgba(129, 199, 132, 0.08);
  border: 1px solid rgba(129, 199, 132, 0.18);
}
.tool-active-label i { font-size: 13px; }
.tool-progress {
  flex-basis: 100%;
  height: 5px;
  background: rgba(255, 255, 255, 0.06);
  border-radius: 100px;
  overflow: hidden;
}
.tool-progress-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--accent), rgba(var(--accent-rgb), 0.6));
  border-radius: 100px;
  box-shadow: 0 0 10px rgba(var(--accent-rgb), 0.25);
  transition: width 0.15s linear;
}
.runtime-item.tool-item-active {
  border-color: rgba(129, 199, 132, 0.24);
  background: linear-gradient(135deg, rgba(129, 199, 132, 0.09), rgba(129, 199, 132, 0.02));
}
.tool-section-label {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 2px 0;
  color: var(--text-dim);
}
.tool-section-label span {
  flex: 0 0 auto;
  font-size: 10px;
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 0.08em;
}
.tool-section-label::after {
  content: '';
  flex: 1;
  height: 1px;
  background: linear-gradient(90deg, var(--border-strong), transparent);
}
@media (max-width: 640px) {
  .tools-header-top { flex-wrap: wrap; }
  .tools-summary { margin-left: 0; }
  .tool-actions { width: 100%; justify-content: flex-start; }
}

/* Modal content */
.media-modal-url {
  font-family: monospace;
  font-size: 11px;
  color: var(--text-dim);
  background: rgba(255,255,255,0.02);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px 10px;
  word-break: break-all;
}

.media-steps {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0;
  padding: 2px 0;
}
.media-step {
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--text-dim);
  font-size: 11.5px;
  font-weight: 600;
  transition: color 0.2s;
}
.media-step.active { color: var(--accent-strong); }
.media-step.done { color: var(--accent-strong); opacity: 0.6; }
.media-step-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--border-strong);
  border: 2px solid var(--border-strong);
  transition: all 0.2s;
}
.media-step.active .media-step-dot {
  background: var(--accent);
  border-color: var(--accent-strong);
  box-shadow: 0 0 8px rgba(var(--accent-rgb), 0.4);
}
.media-step.done .media-step-dot {
  background: var(--accent);
  border-color: var(--accent-strong);
}
.media-step-line {
  width: 36px;
  height: 2px;
  background: var(--border-strong);
  margin: 0 8px;
  transition: background 0.2s;
}
.media-step-line.done { background: var(--accent); }

.media-center {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 6px 0 2px;
  gap: 10px;
}
.media-center-text { color: var(--text-dim); font-size: 13px; font-weight: 500; }
.media-big-spinner {
  width: 30px;
  height: 30px;
  border: 3px solid rgba(var(--accent-rgb), 0.15);
  border-top-color: var(--accent-strong);
  border-radius: 50%;
  animation: ndSpin 0.8s linear infinite;
}
.media-spinner {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
  border: 2px solid rgba(var(--accent-rgb), 0.15);
  border-top-color: var(--accent-strong);
  border-radius: 50%;
  animation: ndSpin 0.8s linear infinite;
}
.media-check {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: rgba(129, 199, 132, 0.12);
  color: var(--status-complete);
  display: flex;
  align-items: center;
  justify-content: center;
}
.media-check .material-icons { font-size: 24px; }

.media-format-select {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.media-format-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  max-height: 210px;
  overflow-y: auto;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: rgba(255,255,255,0.015);
  padding: 4px;
}
.media-format-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 10px;
  border-radius: 8px;
  cursor: pointer;
  border: 1px solid transparent;
  transition: all 0.12s ease;
}
.media-format-item:hover { background: rgba(255,255,255,0.04); }
.media-format-item.selected {
  background: rgba(var(--accent-rgb), 0.08);
  border-color: rgba(var(--accent-rgb), 0.18);
}
.media-format-radio { accent-color: var(--accent-strong); flex-shrink: 0; }
.media-format-text {
  font-size: 12.5px;
  color: var(--text-muted);
  flex: 1;
}
.media-format-size {
  display: block;
  margin-top: 2px;
  font-size: 10.5px;
  color: var(--text-dim);
  font-weight: 500;
}
.media-format-item.selected .media-format-text { color: var(--text); }

.media-format-tag {
  flex-shrink: 0;
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.04em;
  text-transform: uppercase;
  padding: 2px 6px;
  border-radius: 100px;
  white-space: nowrap;
}
.media-format-tag.tag-merger {
  color: var(--accent-strong);
  background: rgba(var(--accent-rgb), 0.1);
  border: 1px solid rgba(var(--accent-rgb), 0.25);
}
.media-format-tag.tag-direct {
  color: var(--text-dim);
  background: var(--hover-bg);
  border: 1px solid var(--border);
}
.media-merger-hint {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 10px;
  font-weight: 600;
  color: #ffb74d;
  background: rgba(255, 152, 0, 0.08);
  border: 1px solid rgba(255, 152, 0, 0.2);
  border-radius: 100px;
  padding: 3px 9px;
  white-space: nowrap;
}
.media-merger-hint i { font-size: 12px; }

.media-error-box {
  background: rgba(239, 83, 80, 0.05);
  border: 1px solid rgba(239, 83, 80, 0.18);
  border-radius: 10px;
  padding: 12px;
}
.media-error-header {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--status-error);
  font-weight: 700;
  font-size: 12.5px;
  margin-bottom: 8px;
}
.media-error-header .material-icons { font-size: 18px; }
.media-error-pre {
  margin: 0;
  font-family: monospace;
  font-size: 11px;
  color: var(--status-error);
  background: rgba(239, 83, 80, 0.06);
  padding: 8px 10px;
  border-radius: 8px;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 120px;
  overflow-y: auto;
}

.media-log-section {
  border: 1px solid var(--border);
  border-radius: 10px;
  background: rgba(255,255,255,0.015);
  overflow: hidden;
}
.media-log-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 7px 10px;
  border-bottom: 1px solid var(--border);
  background: rgba(255,255,255,0.02);
}
.media-log-title {
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-dim);
  opacity: 0.7;
}
.media-log-clear {
  background: none;
  border: none;
  color: var(--text-dim);
  font-size: 10.5px;
  cursor: pointer;
  font-family: inherit;
  transition: color 0.15s;
}
.media-log-clear:hover { color: var(--text); }
.media-log-box {
  max-height: 150px;
  overflow-y: auto;
  padding: 8px 10px;
  font-family: monospace;
  font-size: 10.5px;
  line-height: 1.5;
}
.media-log-box pre { margin: 0; white-space: pre-wrap; word-break: break-word; }
.media-log-line { color: var(--text-dim); }
.media-log-empty { color: var(--text-dim); font-style: italic; font-size: 10.5px; }

/* Footer buttons */
.nd-footer {
  position: relative;
  z-index: 1;
  display: flex;
  gap: 8px;
  padding: 10px 16px 14px;
  border-top: 1px solid var(--border);
  flex-shrink: 0;
}
.nd-btn {
  flex: 1;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  border: none;
  font-weight: 700;
  padding: 10px 16px;
  border-radius: 10px;
  cursor: pointer;
  font-size: 12.5px;
  transition: all 0.15s ease;
  font-family: inherit;
  min-width: 0;
}
.nd-btn i { font-size: 16px; }
.nd-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.nd-btn-primary {
  background: linear-gradient(135deg, var(--accent), rgba(var(--accent-rgb), 0.85));
  color: #0a0e1a;
  box-shadow: 0 2px 6px rgba(var(--accent-rgb), 0.2), inset 0 1px 0 rgba(255,255,255,0.15);
}
.nd-btn-primary:hover:not(.disabled) {
  box-shadow: 0 3px 12px rgba(var(--accent-rgb), 0.3), inset 0 1px 0 rgba(255,255,255,0.2);
  transform: translateY(-1px);
}
.nd-btn-primary:active:not(.disabled) {
  transform: scale(0.98);
  box-shadow: 0 1px 4px rgba(var(--accent-rgb), 0.15);
}
.nd-btn-secondary {
  background: rgba(255,255,255,0.03);
  color: var(--text-muted);
  border: 1px solid var(--border);
}
.nd-btn-secondary:hover {
  background: rgba(var(--accent-rgb), 0.08);
  color: var(--accent-strong);
  border-color: rgba(var(--accent-rgb), 0.2);
}
.nd-btn-secondary:active { transform: scale(0.98); }
.nd-btn-danger {
  background: rgba(239, 83, 80, 0.08);
  color: var(--status-error);
  border: 1px solid rgba(239, 83, 80, 0.2);
}
.nd-btn-danger:hover {
  background: rgba(239, 83, 80, 0.16);
  color: #ff8a80;
  border-color: rgba(239, 83, 80, 0.32);
}
.nd-btn-danger:active { transform: scale(0.98); }

.nd-spinner { animation: ndSpin 0.7s linear infinite; }
@keyframes ndSpin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }
.spinning { animation: ndSpin 0.8s linear infinite; }

.modal-fade-enter-active { transition: opacity 0.2s; }
.modal-fade-leave-active { transition: opacity 0.15s; }
.modal-fade-enter-from, .modal-fade-leave-to { opacity: 0; }

@media (max-width: 520px) {
  .nd-overlay { padding: 0; align-items: flex-end; }
  .nd-modal {
    max-width: 100%;
    border-radius: 20px 20px 0 0;
    animation: ndSlideUp 0.3s cubic-bezier(0.16, 1, 0.3, 1);
  }
  @keyframes ndSlideUp {
    from { opacity: 0; transform: translateY(100%); }
    to   { opacity: 1; transform: translateY(0); }
  }
  .nd-body { padding: 14px 16px 8px; }
  .nd-header { padding: 12px 12px 8px; }
  .nd-divider { margin: 0 12px; }
  .nd-footer { padding: 8px 12px 12px; }
  .nd-card-body { padding: 12px 12px 8px; }
  .nd-card-header { padding: 12px 12px 8px; }
  .media-footer-primary { min-width: 0; }
  .media-resume { margin: 0 12px 12px; }
}
</style>
