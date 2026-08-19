<!-- Downloads page -- list/grid view, add download dialog, schedule modal -->
<template>
  <div class="downloads-page">
    <!-- Hero Section (copied from SettingsPage) -->
    <section class="settings-hero">
      <div class="hero-left">
        <div class="hero-icon-area">
          <div class="hero-orb"></div>
          <i class="material-icons hero-main-icon">cloud_download</i>
        </div>
        <div class="hero-copy">
          <h1>{{ t('Downloads') }}</h1>
          <div class="filter-bar downloads-filter-bar" :class="'pos-' + downloadFilterIndex">
            <span class="filter-btn-indicator"></span>
            <button :class="['filter-btn', { active: downloadFilter === 'all' }]" @click="downloadFilter = 'all'" :title="t('All Downloads')">
              <i class="material-icons">inbox</i>
            </button>
            <button :class="['filter-btn', { active: downloadFilter === 'downloading' }]" @click="downloadFilter = 'downloading'" :title="t('Downloading')">
              <i class="material-icons">download</i>
            </button>
            <button :class="['filter-btn', { active: downloadFilter === 'completed' }]" @click="downloadFilter = 'completed'" :title="t('Completed')">
              <i class="material-icons">done</i>
            </button>
          </div>
        </div>
      </div>
      <!-- Nested bar box in bottom right corner -->
      <div class="hero-bottom-bar">
        <div class="bar-box">
          <button @click="setViewMode('list')" :class="['btn-action-icon', { active: viewMode === 'list' }]" :title="t('List View')">
            <i class="material-icons">view_list</i>
          </button>
          <button @click="setViewMode('compact')" :class="['btn-action-icon', { active: viewMode === 'compact' }]" :title="t('Compact View')">
            <i class="material-icons">view_stream</i>
          </button>
          <div class="bar-divider"></div>
          <button
            :class="['btn-action-icon', { active: grabEnabled }]"
            @click="toggleGrabEnabled"
            :title="grabEnabled ? t('Disable drag reorder') : t('Enable drag reorder')"
          >
            <i class="material-icons">open_with</i>
          </button>
          <div class="sort-wrapper">
            <button @click="sortMenuOpen = !sortMenuOpen" ref="sortBtnRef" class="btn-action-icon" :title="t('Sort by')">
              <i class="material-icons">sort</i>
            </button>
            <div v-if="sortMenuOpen" class="sort-menu">
              <span class="sort-menu-label">{{ t('Sort by') }}</span>
              <button v-for="opt in sortOptions" :key="opt" @click="setSortOption(opt)" class="sort-menu-item" :class="{ active: sortOption === opt }">
                {{ t(sortLabels[opt]) }}
              </button>
              <div class="sort-menu-divider"></div>
              <span class="sort-menu-label">{{ t('Order') }}</span>
              <button @click="setSortAscending(true)" class="sort-menu-item" :class="{ active: sortAscending }">
                              {{ t('Ascending') }}
                            </button>
                            <button @click="setSortAscending(false)" class="sort-menu-item" :class="{ active: !sortAscending }">
                              {{ t('Descending') }}
                            </button>
            </div>
          </div>
          <div class="bar-divider"></div>
          <button @click="pauseAll" class="btn-action-icon" :title="t('Pause All Downloads')">
            <i class="material-icons">pause</i>
          </button>
          <button @click="resumeAll" class="btn-action-icon" :title="t('Resume All Downloads')">
            <i class="material-icons">play_arrow</i>
          </button>
          <button @click="clearHistory" class="btn-action-icon" :title="t('Clear History')">
            <i class="material-icons">delete_sweep</i>
          </button>
          <div class="bar-divider"></div>
          <button @click="showCreateGroupDialog = true" class="btn-action-icon" :title="t('Add Group')">
            <i class="material-icons">folder_shared</i>
          </button>
          <button @click="showBatchDialog = true" class="btn-action-icon" :title="t('Batch Download')"><i class="material-icons">playlist_add</i></button>
          <button @click="showAddDialog = true" class="btn-add-icon" :title="t('New Download')">
            <i class="material-icons">add</i>
          </button>
        </div>
      </div>
    </section>

    <!-- Sticky toolbar on scroll -->
    <transition name="sticky-fade">
      <div v-if="showStickyToolbar" class="sticky-toolbar">
        <div class="filter-bar" :class="'pos-' + downloadFilterIndex">
          <span class="filter-btn-indicator"></span>
          <button :class="['filter-btn', { active: downloadFilter === 'all' }]" @click="downloadFilter = 'all'" :title="t('All Downloads')">
            <i class="material-icons">inbox</i>
          </button>
          <button :class="['filter-btn', { active: downloadFilter === 'downloading' }]" @click="downloadFilter = 'downloading'" :title="t('Downloading')">
            <i class="material-icons">download</i>
          </button>
          <button :class="['filter-btn', { active: downloadFilter === 'completed' }]" @click="downloadFilter = 'completed'" :title="t('Completed')">
            <i class="material-icons">done</i>
          </button>
        </div>
        <div style="flex:1" data-tauri-drag-region></div>
        <div class="bar-box">
          <button :class="['btn-action-icon', { active: viewMode === 'list' }]" @click="setViewMode('list')" :title="t('List View')">
            <i class="material-icons">view_list</i>
          </button>
          <button :class="['btn-action-icon', { active: viewMode === 'compact' }]" @click="setViewMode('compact')" :title="t('Compact View')">
            <i class="material-icons">view_stream</i>
          </button>
          <button :class="['btn-action-icon', { active: grabEnabled }]" @click="toggleGrabEnabled" :title="grabEnabled ? t('Disable drag reorder') : t('Enable drag reorder')">
            <i class="material-icons">open_with</i>
          </button>
          <div class="sort-wrapper">
            <button @click="sortMenuOpen = !sortMenuOpen" ref="stickySortBtnRef" class="btn-action-icon" :title="t('Sort by')">
              <i class="material-icons">sort</i>
            </button>
            <div v-if="sortMenuOpen" class="sort-menu">
              <span class="sort-menu-label">{{ t('Sort by') }}</span>
              <button v-for="opt in sortOptions" :key="opt" @click="setSortOption(opt)" class="sort-menu-item" :class="{ active: sortOption === opt }">
                {{ t(sortLabels[opt]) }}
              </button>
              <div class="sort-menu-divider"></div>
              <span class="sort-menu-label">{{ t('Order') }}</span>
              <button @click="setSortAscending(true)" class="sort-menu-item" :class="{ active: sortAscending }">
                              {{ t('Ascending') }}
                            </button>
                            <button @click="setSortAscending(false)" class="sort-menu-item" :class="{ active: !sortAscending }">
                              {{ t('Descending') }}
              </button>
            </div>
          </div>
          <div class="bar-divider"></div>
          <button @click="pauseAll" class="btn-action-icon" :title="t('Pause All Downloads')">
            <i class="material-icons">pause</i>
          </button>
          <button @click="resumeAll" class="btn-action-icon" :title="t('Resume All Downloads')">
            <i class="material-icons">play_arrow</i>
          </button>
          <button @click="clearHistory" class="btn-action-icon" :title="t('Clear History')">
            <i class="material-icons">delete_sweep</i>
          </button>
          <div class="bar-divider"></div>
          <button @click="showCreateGroupDialog = true" class="btn-action-icon" :title="t('Add Group')">
            <i class="material-icons">folder_shared</i>
          </button>
          <button @click="showBatchDialog = true" class="btn-action-icon" :title="t('Batch Download')"><i class="material-icons">playlist_add</i></button>
          <button @click="showAddDialog = true" class="btn-add-icon" :title="t('New Download')">
            <i class="material-icons">add</i>
          </button>
        </div>
      </div>
    </transition>

    <!-- Empty state when no downloads match filter -->
    <div v-if="groupedDownloads.length === 0" class="empty-state">
      <i class="material-icons">cloud_download</i>
      <p>{{ t('No downloads yet') }}</p>
      <span>{{ t('Click "New Download" to add your first download') }}</span>
    </div>

    <!-- â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
         LIST VIEW â€” rendered when viewMode === 'list'
         Full-width cards with filename, status, progress, actions
         â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â• -->
    <div v-if="viewMode === 'list'" ref="listContainerRef" class="downloads-list drag-list-container" :class="{ 'grab-mode': grabEnabled }">
      <template v-for="item in groupedDownloads" :key="(item.type === 'group' ? 'group-' + item.group.id : item.download.gid) + '|' + listKey">
        <div v-if="item.type === 'group'" class="group-container" :class="{ 'group-collapsed': item.group.collapsed }" :id="'group-' + item.group.id" :style="{ '--group-color': item.group.color }">
          <div v-if="dragOverGroupId === item.group.id" class="group-drop-overlay">
            <i class="material-icons">file_download</i>
            <span>{{ t('Drop here') }}</span>
          </div>
          <div class="group-header" @click="toggleGroupCollapse(item.group.id)">
            <div class="group-header-top">
              <div class="group-header-left">
                <span class="group-avatar">
                  <i class="material-icons">folder</i>
                </span>
                <div class="group-heading">
                  <div class="group-title-row">
                    <span class="group-name">{{ item.group.name }}</span>
                    <i class="material-icons group-toggle-icon" :class="{ rotated: item.group.collapsed }">expand_more</i>
                  </div>
                  <div class="group-meta">
                    <span class="group-count"><i class="material-icons">inbox</i>{{ item.downloads.length }}</span>
                    <span class="group-size"><i class="material-icons">storage</i>{{ parseGroupSize(item.downloads) }}</span>
                    <span class="group-added"><i class="material-icons">calendar_today</i>{{ formatGroupDate(item.dateAdded) }}</span>
                    <span v-if="item.group.downloadLimitEnabled && item.group.downloadLimitBytes > 0" class="group-limit-badge" :title="formatGroupLimitDisplay(item.group.downloadLimitUsedBytes) + ' / ' + formatGroupLimitDisplay(item.group.downloadLimitBytes)">
                      <i class="material-icons">speed</i>{{ formatGroupLimitDisplay(item.group.downloadLimitUsedBytes) }} / {{ formatGroupLimitDisplay(item.group.downloadLimitBytes) }}
                    </span>
                  </div>
                </div>
              </div>
              <div class="group-header-right">
                <button class="group-action-btn" @click.stop="openEditGroup(item.group)"><i class="material-icons">edit</i></button>
                <button class="group-action-btn" @click.stop="deleteGroup(item.group.id)"><i class="material-icons">delete</i></button>
              </div>
            </div>
            <transition name="group-preview-fade">
              <div v-if="item.group.collapsed" class="group-preview">
                <div v-if="item.downloads.length === 0" class="group-preview-empty">
                  <i class="material-icons">inbox</i>
                  <span>{{ t('No downloads in this group yet') }}</span>
                </div>
                <div v-for="dl in item.downloads.slice(0, 3)" :key="dl.download.gid" class="group-preview-row">
                  <div class="group-preview-row-top">
                    <span class="group-preview-name" :title="dl.download.filename">{{ dl.download.filename }}</span>
                    <span class="group-preview-size">{{ dl.download.size }}</span>
                  </div>
                  <div class="group-preview-track">
                    <div class="group-preview-fill" :style="{ width: Math.min(100, Math.max(0, dl.download.progress || 0)) + '%' }"></div>
                  </div>
                </div>
                <div v-if="item.downloads.length > 3" class="group-preview-more">+{{ item.downloads.length - 3 }} {{ t('more') }}</div>
              </div>
            </transition>
            <div class="group-progress"><div class="group-progress-fill" :style="{ width: parseGroupProgress(item.downloads) + '%' }"></div></div>
          </div>
          <div class="group-body" :class="{ collapsed: item.group.collapsed }" :style="{ '--group-color': item.group.color }">
            <transition name="group-body-fade">
              <div v-if="!item.group.collapsed" class="group-downloads drag-list-container" :class="{ 'grab-active': grabEnabled }">
              <DragItem
                v-for="dl in item.downloads"
                :key="dl.download.gid"
                :gid="dl.download.gid"
              >
                <DownloadListCard
                  :download="dl.download"
                  :get-requested-connections="getRequestedConnections"
                  :get-status-text-style="getStatusTextStyle"
                  :get-status-icon="getStatusIcon"
                  :capitalize-status="capitalizeStatus"
                  :show-download-details="showDownloadDetails"
                  :show-error-details="showErrorDetails"
                  :pause-download="pauseDownload"
                  :resume-download="resumeDownload"
                  :retry-download="retryDownload"
                  :redownload-file="redownloadFile"
                  :show-in-folder="showInFolder"
                  :open-download-file="openDownloadFile"
                  :remove-download="removeDownload"
                  :cancel-download="cancelDownload"
                  :groups="groups"
                  :add-to-group="addToGroup"
                />
              </DragItem>
              </div>
            </transition>
          </div>
        </div>
        <DragItem v-else
          :gid="item.download.gid"
        >
          <DownloadListCard
            :download="item.download"
            :get-requested-connections="getRequestedConnections"
            :get-status-text-style="getStatusTextStyle"
            :get-status-icon="getStatusIcon"
            :capitalize-status="capitalizeStatus"
            :show-download-details="showDownloadDetails"
            :show-error-details="showErrorDetails"
            :pause-download="pauseDownload"
            :resume-download="resumeDownload"
            :retry-download="retryDownload"
            :redownload-file="redownloadFile"
            :show-in-folder="showInFolder"
            :open-download-file="openDownloadFile"
            :remove-download="removeDownload"
                  :cancel-download="cancelDownload"
            :groups="[]"
            :add-to-group="() => {}"
          />
        </DragItem>
      </template>
      <div v-if="grabEnabled && hasNoUngroupedDownloads" class="top-level-drop-zone" :class="{ 'is-active': sorting }">
        <i class="material-icons">arrow_upward</i>
        <span>{{ t('Drop here to remove from group') }}</span>
      </div>
    </div>

    <!-- â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
         COMPACT LIST â€” rendered when viewMode === 'compact'
         Single-row per download, tight spacing, minimal chrome
         â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â• -->
    <div v-else-if="viewMode === 'compact'" ref="compactContainerRef" class="downloads-compact drag-list-container" :class="{ 'grab-mode': grabEnabled }">
      <template v-for="item in groupedDownloads" :key="(item.type === 'group' ? 'group-' + item.group.id : item.download.gid) + '|' + listKey">
        <div v-if="item.type === 'group'" class="group-container compact-group" :class="{ 'group-collapsed': item.group.collapsed }" :id="'group-' + item.group.id" :style="{ '--group-color': item.group.color }">
          <div v-if="dragOverGroupId === item.group.id" class="group-drop-overlay">
            <i class="material-icons">file_download</i>
            <span>{{ t('Drop here') }}</span>
          </div>
          <div class="group-header" @click="toggleGroupCollapse(item.group.id)">
            <div class="group-header-top">
              <div class="group-header-left">
                <span class="group-avatar">
                  <i class="material-icons">folder</i>
                </span>
                <div class="group-heading">
                  <div class="group-title-row">
                    <span class="group-name">{{ item.group.name }}</span>
                    <i class="material-icons group-toggle-icon" :class="{ rotated: item.group.collapsed }">expand_more</i>
                  </div>
                  <div class="group-meta">
                    <span class="group-count"><i class="material-icons">inbox</i>{{ item.downloads.length }}</span>
                    <span class="group-size"><i class="material-icons">storage</i>{{ parseGroupSize(item.downloads) }}</span>
                    <span class="group-added"><i class="material-icons">calendar_today</i>{{ formatGroupDate(item.dateAdded) }}</span>
                    <span v-if="item.group.downloadLimitEnabled && item.group.downloadLimitBytes > 0" class="group-limit-badge" :title="formatGroupLimitDisplay(item.group.downloadLimitUsedBytes) + ' / ' + formatGroupLimitDisplay(item.group.downloadLimitBytes)">
                      <i class="material-icons">speed</i>{{ formatGroupLimitDisplay(item.group.downloadLimitUsedBytes) }} / {{ formatGroupLimitDisplay(item.group.downloadLimitBytes) }}
                    </span>
                  </div>
                </div>
              </div>
              <div class="group-header-right">
                <button class="group-action-btn" @click.stop="openEditGroup(item.group)"><i class="material-icons">edit</i></button>
                <button class="group-action-btn" @click.stop="deleteGroup(item.group.id)"><i class="material-icons">delete</i></button>
              </div>
            </div>
            <div class="group-progress"><div class="group-progress-fill" :style="{ width: parseGroupProgress(item.downloads) + '%' }"></div></div>
          </div>
          <div class="group-body" :class="{ collapsed: item.group.collapsed }" :style="{ '--group-color': item.group.color }">
            <transition name="group-body-fade">
              <div v-if="!item.group.collapsed" class="group-downloads drag-list-container" :class="{ 'grab-active': grabEnabled }">
                <DragItem
                  v-for="dl in item.downloads"
                  :key="dl.download.gid"
                  :gid="dl.download.gid"
                >
                  <DownloadListCardCompact
                    :download="dl.download"
                    :get-requested-connections="getRequestedConnections"
                    :get-status-text-style="getStatusTextStyle"
                    :get-status-icon="getStatusIcon"
                    :capitalize-status="capitalizeStatus"
                    :show-download-details="showDownloadDetails"
                    :show-error-details="showErrorDetails"
                    :pause-download="pauseDownload"
                    :resume-download="resumeDownload"
                    :retry-download="retryDownload"
                    :redownload-file="redownloadFile"
                    :show-in-folder="showInFolder"
                    :open-download-file="openDownloadFile"
                    :remove-download="removeDownload"
                    :cancel-download="cancelDownload"
                    :groups="[]"
                    :add-to-group="() => {}"
                  />
                </DragItem>
              </div>
            </transition>
          </div>
        </div>
        <DragItem v-else
          :gid="item.download.gid"
        >
          <DownloadListCardCompact
            :download="item.download"
            :get-requested-connections="getRequestedConnections"
            :get-status-text-style="getStatusTextStyle"
            :get-status-icon="getStatusIcon"
            :capitalize-status="capitalizeStatus"
            :show-download-details="showDownloadDetails"
            :show-error-details="showErrorDetails"
            :pause-download="pauseDownload"
            :resume-download="resumeDownload"
            :retry-download="retryDownload"
            :redownload-file="redownloadFile"
            :show-in-folder="showInFolder"
            :open-download-file="openDownloadFile"
            :remove-download="removeDownload"
                  :cancel-download="cancelDownload"
            :groups="[]"
            :add-to-group="() => {}"
          />
        </DragItem>
      </template>
      <div v-if="grabEnabled && hasNoUngroupedDownloads" class="top-level-drop-zone" :class="{ 'is-active': sorting }">
        <i class="material-icons">arrow_upward</i>
        <span>{{ t('Drop here to remove from group') }}</span>
      </div>
    </div>

    <!-- Create Group Dialog -->
    <transition name="modal-fade">
      <div
        v-if="showCreateGroupDialog"
        class="nd-overlay"
        @mousedown.self.prevent="closeOutsidePress()"
        @mouseup.self="closeOutsideRelease('create')"
        @mouseleave="closeOutsideCancel()"
      >
        <transition name="nd-slide">
          <div class="nd-modal group-nd-modal" @click.stop>
            <div class="nd-glow"></div>

            <header class="nd-header">
              <div class="nd-header-left">
                <div class="nd-avatar">
                  <i class="material-icons">folder_shared</i>
                </div>
                <div class="nd-titles">
                  <h1 class="nd-title">{{ t('Create Group') }}</h1>
                  <span class="nd-subtitle">{{ t('Group') }}</span>
                </div>
              </div>
              <div class="nd-header-actions">
                <button
                  class="nd-icon-btn group-options-toggle"
                  :class="{ active: showGroupOptions }"
                  @click="showGroupOptions = !showGroupOptions"
                  :title="showGroupOptions ? t('Hide options') : t('Show options')"
                >
                  <i class="material-icons" :class="{ rotated: !showGroupOptions }">tune</i>
                </button>
                <button class="nd-icon-btn nd-close" @click="showCreateGroupDialog = false" :aria-label="t('Close')" :title="t('Close (Esc)')">
                  <i class="material-icons">close</i>
                </button>
              </div>
            </header>

            <div class="nd-divider"></div>

            <div class="nd-body">
              <div class="nd-section">
                <div class="nd-section-header">
                  <h3 class="nd-section-title">{{ t('Details') }}</h3>
                </div>
                <div class="nd-field">
                  <label class="nd-label">
                    <i class="material-icons nd-label-icon">badge</i>
                    {{ t('Group Name') }}
                  </label>
                  <input
                    ref="groupNameInput"
                    v-model="newGroupName"
                    class="nd-input"
                    :placeholder="t('Group name placeholder')"
                    @keyup.enter="confirmCreateGroup"
                  />
                </div>
                <div class="nd-field">
                  <label class="nd-label">
                    <i class="material-icons nd-label-icon">palette</i>
                    {{ t('Color') }}
                  </label>
                  <div class="color-picker-row">
                    <button
                      v-for="c in groupColorOptions"
                      :key="c"
                      class="color-swatch"
                      :class="{ selected: newGroupColor === c }"
                      :style="{ background: c }"
                      @click="newGroupColor = c"
                    />
                  </div>
                </div>
                <button class="group-downloads-btn nd-btn-secondary" @click="openGroupDownloads('new')">
                  <i class="material-icons">playlist_add</i>
                  <span>{{ t('Downloads') }}</span>
                  <span v-if="newGroupMembers.length" class="group-member-count">{{ newGroupMembers.length }}</span>
                </button>
              </div>

              <template v-if="showGroupOptions">
                <div class="nd-section">
                  <div class="nd-section-header">
                    <h3 class="nd-section-title">{{ t('Options') }}</h3>
                  </div>
                  <div class="nd-field">
                    <label class="nd-label">
                      <i class="material-icons nd-label-icon">done_all</i>
                      {{ t('When all downloads complete') }}
                    </label>
                    <CustomDropdown v-model="newGroupOnComplete" :options="onCompleteOptions" />
                  </div>
                  <div class="nd-field">
                    <label class="nd-label">
                      <i class="material-icons nd-label-icon">speed</i>
                      {{ t('Speed Limit') }}
                    </label>
                    <div class="nd-toggle-row">
                      <div class="speed-toggle" :class="{ on: newGroupSpeedLimit > 0 }" @click="newGroupSpeedLimit = newGroupSpeedLimit > 0 ? 0 : 200">
                        <div class="speed-toggle-knob"></div>
                      </div>
                      <span class="nd-toggle-label">{{ newGroupSpeedLimit > 0 ? t('On') : t('Off') }}</span>
                      <span class="nd-toggle-hint">{{ newGroupSpeedLimit > 0 ? newGroupSpeedLimit + ' KB/s' : t('Unlimited') }}</span>
                    </div>
                    <transition name="speed-slide">
                      <div v-if="newGroupSpeedLimit > 0" class="stepper">
                        <button @click="newGroupSpeedLimit = Math.max(100, newGroupSpeedLimit - 100)" :disabled="newGroupSpeedLimit <= 100"><i class="material-icons">remove</i></button>
                        <input v-model.number="newGroupSpeedLimit" type="number" min="100" step="100" @change="newGroupSpeedLimit = Math.max(100, Number(newGroupSpeedLimit) || 100)" />
                        <button @click="newGroupSpeedLimit += 100"><i class="material-icons">add</i></button>
                      </div>
                    </transition>
                  </div>
                  <div class="field">
                      <label class="nd-label">
                        <i class="material-icons nd-label-icon">data_usage</i>
                        {{ t('Download Limit') }}
                      </label>
                      <div class="speed-toggle-row">
                        <div class="speed-toggle" :class="{ on: newGroupDlLimitEnabled }" @click="newGroupDlLimitEnabled = !newGroupDlLimitEnabled; if (newGroupDlLimitEnabled && newGroupDlLimitBytes === 0) { newGroupDlLimitBytes = 1048576; newGroupDlLimitInputValue = 1; newGroupDlLimitUnit = 'MB' }">
                          <div class="speed-toggle-knob"></div>
                        </div>
                        <span class="speed-toggle-label">{{ newGroupDlLimitEnabled ? t('On') : t('Off') }}</span>
                        <span class="speed-toggle-hint" v-if="!newGroupDlLimitEnabled">{{ t('Unlimited') }}</span>
                        <span class="speed-toggle-hint" v-else>{{ formatGroupLimitDisplay(newGroupDlLimitBytes) }}</span>
                      </div>
                    </div>
                    <transition name="speed-slide">
                      <div v-if="newGroupDlLimitEnabled" class="field">
                        <label>{{ t('Limit') }}</label>
                        <div class="nd-input-wrap">
                          <input v-model.number="newGroupDlLimitInputValue" type="number" min="1" @input="newGroupDlLimitBytes = groupDlBytesFromInput(newGroupDlLimitInputValue, newGroupDlLimitUnit)" class="nd-input nd-path-input" placeholder="e.g. 1" />
                          <CustomDropdown v-model="newGroupDlLimitUnit" :options="groupDlLimitUnitOptions" class="dl-limit-unit-custom" @update:modelValue="newGroupDlLimitBytes = groupDlBytesFromInput(newGroupDlLimitInputValue, newGroupDlLimitUnit)" />
                        </div>
                        <span class="field-hint">{{ t('Equivalent to') }} {{ formatGroupLimitDisplay(newGroupDlLimitBytes) }}</span>
                      </div>
                    </transition>
                    <div v-if="newGroupDlLimitEnabled && newGroupDlLimitBytes > 0" class="field">
                      <label>{{ t('Usage') }}</label>
                      <div class="limit-usage-bar">
                        <div class="limit-usage-track">
                          <div class="limit-usage-fill" :style="{ width: '0%' }"></div>
                        </div>
                        <span class="limit-usage-text">0 / {{ formatGroupLimitDisplay(newGroupDlLimitBytes) }}</span>
                      </div>
                    </div>
                </div>
              </template>
            </div>

            <footer class="nd-footer">
              <button @click="showCreateGroupDialog = false" class="nd-btn nd-btn-secondary">
                <i class="material-icons">close</i> {{ t('Cancel') }}
              </button>
              <button @click="confirmCreateGroup" class="nd-btn nd-btn-primary">
                <i class="material-icons">add</i> {{ t('Create') }}
              </button>
            </footer>
          </div>
        </transition>
      </div>
    </transition>

    <!-- Rename Group Dialog -->
    <transition name="modal-fade">
      <div
        v-if="renameGroupId"
        class="nd-overlay"
        @mousedown.self.prevent="closeOutsidePress()"
        @mouseup.self="closeOutsideRelease('rename')"
        @mouseleave="closeOutsideCancel()"
      >
        <transition name="nd-slide">
          <div class="nd-modal group-nd-modal" @click.stop>
            <div class="nd-glow"></div>

            <header class="nd-header">
              <div class="nd-header-left">
                <div class="nd-avatar">
                  <i class="material-icons">edit</i>
                </div>
                <div class="nd-titles">
                  <h1 class="nd-title">{{ t('Edit Group') }}</h1>
                  <span class="nd-subtitle">{{ t('Group') }}</span>
                </div>
              </div>
              <div class="nd-header-actions">
                <button
                  class="nd-icon-btn group-options-toggle"
                  :class="{ active: showGroupOptions }"
                  @click="showGroupOptions = !showGroupOptions"
                  :title="showGroupOptions ? t('Hide options') : t('Show options')"
                >
                  <i class="material-icons" :class="{ rotated: !showGroupOptions }">tune</i>
                </button>
                <button class="nd-icon-btn nd-close" @click="renameGroupId = null" :aria-label="t('Close')" :title="t('Close (Esc)')">
                  <i class="material-icons">close</i>
                </button>
              </div>
            </header>

            <div class="nd-divider"></div>

            <div class="nd-body">
              <div class="nd-section">
                <div class="nd-section-header">
                  <h3 class="nd-section-title">{{ t('Details') }}</h3>
                </div>
                <div class="nd-field">
                  <label class="nd-label">
                    <i class="material-icons nd-label-icon">badge</i>
                    {{ t('Group Name') }}
                  </label>
                  <input
                    v-model="renameGroupName"
                    class="nd-input"
                    :placeholder="t('Group name...')"
                    @keyup.enter="confirmRenameGroup"
                  />
                </div>
                <div class="nd-field">
                  <label class="nd-label">
                    <i class="material-icons nd-label-icon">palette</i>
                    {{ t('Color') }}
                  </label>
                  <div class="color-picker-row">
                    <button
                      v-for="c in groupColorOptions"
                      :key="c"
                      class="color-swatch"
                      :class="{ selected: renameGroupColor === c }"
                      :style="{ background: c }"
                      @click="renameGroupColor = c"
                    />
                  </div>
                </div>
                <button class="group-downloads-btn nd-btn-secondary" @click="openGroupDownloads('edit')">
                  <i class="material-icons">playlist_add</i>
                  <span>{{ t('Downloads') }}</span>
                  <span v-if="editingGroupMembers.length" class="group-member-count">{{ editingGroupMembers.length }}</span>
                </button>
              </div>

              <template v-if="showGroupOptions">
                <div class="nd-section">
                  <div class="nd-section-header">
                    <h3 class="nd-section-title">{{ t('Options') }}</h3>
                  </div>
                  <div class="nd-field">
                    <label class="nd-label">
                      <i class="material-icons nd-label-icon">done_all</i>
                      {{ t('When all downloads complete') }}
                    </label>
                    <CustomDropdown v-model="renameGroupOnComplete" :options="onCompleteOptions" />
                  </div>
                  <div class="nd-field">
                    <label class="nd-label">
                      <i class="material-icons nd-label-icon">speed</i>
                      {{ t('Speed Limit') }}
                    </label>
                    <div class="nd-toggle-row">
                      <div class="speed-toggle" :class="{ on: renameGroupSpeedLimit > 0 }" @click="renameGroupSpeedLimit = renameGroupSpeedLimit > 0 ? 0 : 200">
                        <div class="speed-toggle-knob"></div>
                      </div>
                      <span class="nd-toggle-label">{{ renameGroupSpeedLimit > 0 ? t('On') : t('Off') }}</span>
                      <span class="nd-toggle-hint">{{ renameGroupSpeedLimit > 0 ? renameGroupSpeedLimit + ' KB/s' : t('Unlimited') }}</span>
                    </div>
                    <transition name="speed-slide">
                      <div v-if="renameGroupSpeedLimit > 0" class="stepper">
                        <button @click="renameGroupSpeedLimit = Math.max(100, renameGroupSpeedLimit - 100)" :disabled="renameGroupSpeedLimit <= 100"><i class="material-icons">remove</i></button>
                        <input v-model.number="renameGroupSpeedLimit" type="number" min="100" step="100" @change="renameGroupSpeedLimit = Math.max(100, Number(renameGroupSpeedLimit) || 100)" />
                        <button @click="renameGroupSpeedLimit += 100"><i class="material-icons">add</i></button>
                      </div>
                    </transition>
                  </div>
                  <div class="field">
                      <label class="nd-label">
                        <i class="material-icons nd-label-icon">data_usage</i>
                        {{ t('Download Limit') }}
                      </label>
                      <div class="speed-toggle-row">
                        <div class="speed-toggle" :class="{ on: renameGroupDlLimitEnabled }" @click="renameGroupDlLimitEnabled = !renameGroupDlLimitEnabled; if (renameGroupDlLimitEnabled && renameGroupDlLimitBytes === 0) { renameGroupDlLimitBytes = 1048576; renameGroupDlLimitInputValue = 1; renameGroupDlLimitUnit = 'MB' }">
                          <div class="speed-toggle-knob"></div>
                        </div>
                        <span class="speed-toggle-label">{{ renameGroupDlLimitEnabled ? t('On') : t('Off') }}</span>
                        <span class="speed-toggle-hint" v-if="!renameGroupDlLimitEnabled">{{ t('Unlimited') }}</span>
                        <span class="speed-toggle-hint" v-else>{{ formatGroupLimitDisplay(renameGroupDlLimitBytes) }}</span>
                      </div>
                    </div>
                    <transition name="speed-slide">
                      <div v-if="renameGroupDlLimitEnabled" class="field">
                        <label>{{ t('Limit') }}</label>
                        <div class="nd-input-wrap">
                          <input v-model.number="renameGroupDlLimitInputValue" type="number" min="1" @input="renameGroupDlLimitBytes = groupDlBytesFromInput(renameGroupDlLimitInputValue, renameGroupDlLimitUnit)" class="nd-input nd-path-input" placeholder="e.g. 1" />
                          <CustomDropdown v-model="renameGroupDlLimitUnit" :options="groupDlLimitUnitOptions" class="dl-limit-unit-custom" @update:modelValue="renameGroupDlLimitBytes = groupDlBytesFromInput(renameGroupDlLimitInputValue, renameGroupDlLimitUnit)" />
                        </div>
                        <span class="field-hint">{{ t('Equivalent to') }} {{ formatGroupLimitDisplay(renameGroupDlLimitBytes) }}</span>
                      </div>
                    </transition>
                    <div v-if="renameGroupDlLimitEnabled && renameGroupDlLimitBytes > 0" class="field">
                      <label>{{ t('Usage') }}</label>
                      <div class="limit-usage-bar">
                        <div class="limit-usage-track">
                          <div class="limit-usage-fill" :style="{ width: Math.min(100, (renameGroupDlLimitUsedBytes / renameGroupDlLimitBytes) * 100) + '%' }"></div>
                        </div>
                        <span class="limit-usage-text">{{ formatGroupLimitDisplay(renameGroupDlLimitUsedBytes) }} / {{ formatGroupLimitDisplay(renameGroupDlLimitBytes) }}</span>
                      </div>
                    </div>
                </div>
              </template>
            </div>

            <footer class="nd-footer">
              <button @click="renameGroupId = null" class="nd-btn nd-btn-secondary">
                <i class="material-icons">close</i> {{ t('Cancel') }}
              </button>
              <button @click="confirmRenameGroup" class="nd-btn nd-btn-primary">
                <i class="material-icons">save</i> {{ t('Save') }}
              </button>
            </footer>
          </div>
        </transition>
      </div>
    </transition>

    <!-- Group Downloads Sub-modal -->
    <transition name="modal-fade">
      <div
        v-if="showGroupDownloads"
        class="nd-overlay"
        style="z-index: 11000;"
        @mousedown.self.prevent="closeOutsidePress()"
        @mouseup.self="closeOutsideRelease('groupDl')"
        @mouseleave="closeOutsideCancel()"
      >
        <transition name="nd-slide">
          <div class="nd-modal group-downloads-modal" @click.stop>
            <div class="nd-glow"></div>

            <header class="nd-header">
              <div class="nd-header-left">
                <div class="nd-avatar">
                  <i class="material-icons">playlist_add</i>
                </div>
                <div class="nd-titles">
                  <h1 class="nd-title">{{ t('Downloads') }}</h1>
                  <span class="nd-subtitle">{{ t('Group') }}</span>
                </div>
              </div>
              <div class="nd-header-actions">
                <button class="nd-icon-btn nd-close" @click="showGroupDownloads = false" :aria-label="t('Close')" :title="t('Close (Esc)')">
                  <i class="material-icons">close</i>
                </button>
              </div>
            </header>

            <div class="nd-divider"></div>

            <div class="nd-body">
              <div class="nd-input-row">
                <input
                  v-model="groupDownloadUrl"
                  class="nd-input"
                  :placeholder="t('Paste download URL...')"
                  @keyup.enter="addGroupDownload"
                />
                <button class="nd-btn-secondary" @click="addGroupDownload" :disabled="!groupDownloadUrl.trim()">
                  <i class="material-icons">add</i>
                </button>
              </div>

              <div class="nd-member-list">
                <div v-if="displayGroupMembers.length === 0" class="nd-member-empty">
                  {{ t('No downloads in this group yet') }}
                </div>
                <div
                  v-for="item in displayGroupMembers"
                  :key="item.gid"
                  class="group-member-row"
                >
                  <div class="group-member-info">
                    <span class="group-member-name">{{ item.filename }}</span>
                    <span class="group-member-status" :class="{ pending: item.isPending }" :style="item.isPending ? '' : getStatusTextStyle(item.status)">
                      {{ item.isPending ? t('Pending') : capitalizeStatus(item.status) }}
                    </span>
                  </div>
                  <button class="group-member-remove" @click="removeGroupMember(item.gid)" :title="t('Remove from group')">
                    <i class="material-icons">close</i>
                  </button>
                </div>
              </div>
            </div>

            <footer class="nd-footer">
              <button @click="showGroupDownloads = false" class="nd-btn nd-btn-primary">
                <i class="material-icons">check</i> {{ t('Done') }}
              </button>
            </footer>
          </div>
        </transition>
      </div>
    </transition>

    <!-- ══════════════════════════════════════════════════════════════════════════════════════
         NEW DOWNLOAD DIALOG — section-based, matches pending-download modal
         ══════════════════════════════════════════════════════════════════════════════════════ -->
    <transition name="modal-fade">
      <div v-if="showAddDialog" class="nd-overlay" @mousedown.self.prevent="startOutsideClose" @mouseup.self="finishOutsideClose" @mouseleave="cancelOutsideClose">
        <transition name="nd-slide">
          <div class="nd-modal" @click.stop>
            <!-- Decorative glow -->
            <div class="nd-glow"></div>

            <!-- Header -->
            <header class="nd-header">
              <div class="nd-header-left">
                <div class="nd-avatar">
                  <i class="material-icons">cloud_download</i>
                </div>
                <div class="nd-titles">
                  <h1 class="nd-title">{{ t('New Download') }}</h1>
                  <span class="nd-subtitle">{{ t('Paste a link and start downloading') }}</span>
                </div>
              </div>
              <div class="nd-header-actions">
                <ProxyUrlCheckBadge :url="addDialogTab === 'url' ? newUrl : ''" />
                <button class="nd-icon-btn nd-close" @click="showAddDialog = false" :aria-label="t('Close')" :title="t('Close (Esc)')">
                  <i class="material-icons">close</i>
                </button>
              </div>
            </header>

            <!-- Divider -->
            <div class="nd-divider"></div>

            <!-- Body -->
            <div class="nd-body">

              <!-- Source Tabs: URL or Torrent -->
              <div class="nd-tabs" role="tablist" :aria-label="t('Download source')">
                <span class="nd-tab-indicator" :class="{ right: addDialogTab === 'torrent' }"></span>
                <button
                  role="tab"
                  class="nd-tab"
                  :class="{ active: addDialogTab === 'url' }"
                  :aria-selected="addDialogTab === 'url'"
                  @click="addDialogTab = 'url'"
                >
                  <i class="material-icons">link</i>
                  <span>{{ t('URL') }}</span>
                </button>
                <button
                  role="tab"
                  class="nd-tab"
                  :class="{ active: addDialogTab === 'torrent' }"
                  :aria-selected="addDialogTab === 'torrent'"
                  @click="addDialogTab = 'torrent'"
                >
                  <i class="material-icons">bolt</i>
                  <span>{{ t('Torrent') }}</span>
                </button>
              </div>

              <!-- Source / URL Section -->
              <div v-if="addDialogTab === 'url'" class="nd-section">
                <div class="nd-section-header">
                  <h3 class="nd-section-title">{{ t('Source') }}</h3>
                  <span v-if="newUrl && !detectedFilename" class="nd-source-badge">{{ detectedHost }}</span>
                </div>
                <div class="nd-input-wrap">
                  <input
                    v-model="newUrl"
                    type="url"
                    class="nd-input"
                    :placeholder="t('URL placeholder')"
                    @keyup.enter="addDownload"
                    autofocus
                  />
                  <button v-if="newUrl" @click="newUrl = ''" class="nd-icon-btn nd-input-clear" :aria-label="t('Clear URL')">
                    <i class="material-icons">close</i>
                  </button>
                </div>
                <span v-if="wasPastedFromClipboard && newUrl" class="nd-clipboard-badge">
                  <i class="material-icons">content_paste</i> {{ t('Pasted from clipboard') }}
                </span>
              </div>

              <!-- Torrent Source Section -->
              <div v-else class="nd-section">
                <div class="nd-section-header">
                  <h3 class="nd-section-title">{{ t('Torrent Source') }}</h3>
                  <span class="nd-source-badge">{{ t('BitTorrent') }}</span>
                </div>

                <!-- .torrent file picker -->
                <div class="nd-field">
                  <label class="nd-label">
                    <i class="material-icons nd-label-icon">description</i>
                    {{ t('Torrent Files') }} <span class="nd-optional">{{ t('(optional if magnet)') }}</span>
                  </label>

                  <!-- Drop zone -->
                  <div
                    ref="dropzoneEl"
                    class="nd-dropzone"
                    :class="{ dragging: isDraggingTorrent, filled: torrentFileList.length > 0 }"
                    @click="pickTorrentFiles"
                    role="button"
                    tabindex="0"
                    :aria-label="torrentFileList.length ? t('Torrent files selected, click to add more') : t('Drop torrent files or click to browse')"
                  >
                    <template v-if="torrentFileList.length > 0">
                      <i class="material-icons nd-dropzone-icon">insert_drive_file</i>
                      <span class="nd-dropzone-name">{{ torrentFileList.length }} {{ t('torrent files selected') }}</span>
                      <span class="nd-dropzone-hint">{{ t('Click to add more or drop more') }}</span>
                    </template>
                    <template v-else>
                      <i class="material-icons nd-dropzone-icon">file_download</i>
                      <span class="nd-dropzone-title">{{ t('Drop .torrent files here') }}</span>
                      <span class="nd-dropzone-hint">{{ t('or click to browse (multi-select)') }}</span>
                    </template>
                  </div>

                  <!-- Torrent list -->
                  <div v-if="torrentFileList.length" class="nd-torrent-list">
                  <div v-for="(torrentItem, ti) in torrentFileList" :key="torrentItem.path" class="nd-torrent-card">
                      <div class="nd-torrent-card-head">
                        <i class="material-icons">description</i>
                        <span class="nd-torrent-card-name" :title="torrentItem.path">{{ torrentItem.name }}</span>
                        <button class="nd-chip-remove" @click="removeTorrentFile(ti)" :aria-label="t('Remove torrent')">
                          <i class="material-icons">close</i>
                        </button>
                      </div>

                      <div v-if="torrentItem.loading" class="nd-torrent-files-loading">
                        <i class="material-icons nd-spinner">hourglass_top</i> {{ t('Reading contents...') }}
                      </div>
                      <div v-else-if="torrentItem.error" class="nd-torrent-files-error">
                        <i class="material-icons">error_outline</i> {{ torrentItem.error }}
                      </div>
                      <div v-else-if="torrentItem.files.length" class="nd-torrent-files">
                        <div class="nd-torrent-files-header">
                          <label class="nd-torrent-select-all">
                            <input type="checkbox" :checked="torrentItem.selected.size === torrentItem.files.length" @change="toggleAllTorrentFilesIn(ti)" />
                            <span>{{ t('Select all') }}</span>
                          </label>
                          <span>{{ torrentItem.selected.size }} / {{ torrentItem.files.length }} · {{ formatTorrentTotal(selectedTorrentFilesOf(torrentItem)) }}</span>
                        </div>
                        <div class="nd-torrent-files-scroll">
                          <label v-for="(f, fi) in torrentItem.files" :key="fi" class="nd-torrent-file-row" :class="{ unchecked: !torrentItem.selected.has(fi) }">
                            <input type="checkbox" :checked="torrentItem.selected.has(fi)" @change="toggleTorrentFileIn(ti, fi)" />
                            <i class="material-icons">insert_drive_file</i>
                            <span class="nd-torrent-file-name" :title="f.path">{{ f.path }}</span>
                            <span class="nd-torrent-file-size">{{ formatFileSize(f.size) }}</span>
                          </label>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>

                <div class="nd-field">
                  <label class="nd-label">
                    <i class="material-icons nd-label-icon">link</i>
                    {{ t('Magnet Link') }} <span class="nd-optional">{{ t('(optional if file)') }}</span>
                  </label>
                  <div class="nd-input-wrap">
                    <input
                      v-model="torrentMagnet"
                      type="text"
                      class="nd-input"
                      placeholder="magnet:?xt=urn:btih:..."
                      @keyup.enter="addTorrent"
                    />
                    <button v-if="torrentMagnet" @click="torrentMagnet = ''" class="nd-icon-btn nd-input-clear" :aria-label="t('Clear magnet')">
                      <i class="material-icons">close</i>
                    </button>
                  </div>
                </div>

                <p class="nd-torrent-hint">
                  <i class="material-icons">info_outline</i>
                  {{ t('Provide a .torrent file or a magnet link — or both. aria2 handles the swarm natively.') }}
                </p>
              </div>

              <!-- Duplicate URL Warning -->
              <Transition name="nd-slide">
                <div v-if="duplicateDownload" class="nd-duplicate-warning">
                  <div class="nd-duplicate-icon">
                    <i class="material-icons">warning</i>
                  </div>
                  <div class="nd-duplicate-info">
                    <span class="nd-duplicate-title">{{ t('Duplicate URL detected') }}</span>
                    <button
                      class="nd-dupe-toggle"
                      role="switch"
                      :aria-checked="allowDuplicate"
                      :aria-label="t('Download as duplicate')"
                      @click="allowDuplicate = !allowDuplicate"
                    >
                      <span class="nd-dupe-track" :class="{ on: allowDuplicate }">
                        <span class="nd-dupe-thumb"></span>
                      </span>
                    </button>
                  </div>
                  <button @click="bringToExistingDownload" class="nd-duplicate-action">
                    <i class="material-icons">visibility</i>
                    {{ t('Show') }}
                  </button>
                </div>
              </Transition>

              <!-- Save As with detected filename -->
              <div class="nd-section">
                <div class="nd-section-header">
                  <h3 class="nd-section-title">{{ t('Save As') }}</h3>
                </div>
                <div class="nd-filename-row">
                  <div class="nd-input-wrap nd-filename-wrap">
                    <input
                      v-model="newFilename"
                      type="text"
                      class="nd-input nd-filename-input"
                      :placeholder="t('Auto-detect from URL')"
                      @keyup.enter="addDownload"
                    />
                    <button
                      v-if="newFilename"
                      @click="newFilename = ''"
                      class="nd-icon-btn nd-input-clear"
                      :aria-label="t('Clear filename')"
                    >
                      <i class="material-icons">close</i>
                    </button>
                  </div>
                </div>
              </div>

              <!-- Save Location -->
              <div class="nd-section">
                <div class="nd-section-header">
                  <h3 class="nd-section-title">{{ t('Save Location') }}</h3>
                </div>
                <div class="nd-input-wrap">
                  <input
                    v-model="newPath"
                    type="text"
                    class="nd-input nd-path-input"
                    :placeholder="t('Save path placeholder')"
                  />
                  <button @click="selectDownloadFolder" class="nd-input-action" :aria-label="t('Browse folders')" :title="t('Browse folder')">
                    <i class="material-icons">folder_open</i>
                  </button>
                </div>
              </div>

              <!-- Connections -->
              <div class="nd-section">
                <div class="nd-section-header">
                  <h3 class="nd-section-title">{{ t('Connections') }}</h3>
                </div>
                <ConnectionSelector v-model="connections" @enter="addDownload" />
              </div>

              <div v-if="addDialogTab === 'url'" class="nd-section">
                <CategoryPicker
                  :enabled="categoriesEnabled"
                  :mode="categoryMode"
                  :selected="selectedCategory"
                  :filename="newFilename || detectedFilename || newUrl"
                  @update:enabled="categoriesEnabled = $event; categoryError = ''"
                  @update:mode="categoryMode = $event; categoryError = ''"
                  @update:selected="selectedCategory = $event; categoryError = ''"
                />
                <div v-if="categoryError" class="nd-category-error">
                  <i class="material-icons">info</i>
                  <span>{{ categoryError }}</span>
                </div>
              </div>

              <!-- Schedule -->
              <div class="nd-section">
                <SchedulePicker
                  v-model:enabled="scheduleEnabled"
                  v-model:schedule="pendingSchedule"
                />
              </div>

              <!-- Proxy -->
              <div class="nd-section">
                <ProxyField v-model="downloadProxy" />
              </div>
            </div>

            <!-- Footer -->
            <footer class="nd-footer">
              <button @click="showAddDialog = false" class="nd-btn nd-btn-secondary">
                <i class="material-icons">close</i> {{ t('Cancel') }}
              </button>
              <button
                @click="addDialogTab === 'url' ? addDownload() : addTorrent()"
                :disabled="isAdding || (addDialogTab === 'url' ? !newUrl.trim() : (torrentFileList.length === 0 && !torrentMagnet.trim()))"
                class="nd-btn nd-btn-primary"
                :class="{ disabled: isAdding || (addDialogTab === 'url' ? !newUrl.trim() : (torrentFileList.length === 0 && !torrentMagnet.trim())) }"
              >
                <i v-if="isAdding" class="material-icons nd-spinner">hourglass_top</i>
                <i v-else class="material-icons">{{ addDialogTab === 'url' ? 'cloud_download' : 'bolt' }}</i>
                <span>{{ isAdding ? t('Starting...') : addDialogTab === 'url' ? t('Start Download') : t('Add Torrent') }}</span>
              </button>
            </footer>

          </div>
        </transition>
      </div>
    </transition>

    <!-- Schedule modal (reused for new download + details view) -->
    <ScheduleModal ref="detailsScheduleModalRef" />

    <!-- Error Details Dialog -->
    <ErrorDetailsDialog
      :visible="showErrorDialog"
      :filename="selectedError.filename"
      :message="selectedError.message"
      @close="showErrorDialog = false"
    />

    <!-- Batch Download Dialog -->
    <BatchUrlDialog
      :visible="showBatchDialog"
      @close="showBatchDialog = false"
    />

    <!-- Hash Verify Dialog -->
    <HashVerifyDialog
      :visible="showHashDialog"
      :filePath="hashFilePath"
      :filename="hashFilename"
      @close="showHashDialog = false"
    />

    <!-- â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•
         DOWNLOAD DETAILS MODAL â€” table layout, modern UX
         â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â•â• -->
    <transition name="modal-fade">
      <div v-if="showDetailsModal" class="details-overlay">
        <transition name="details-scale">
          <div class="details-modal-new" @click.stop>
            <header class="dm-header">
              <div class="dm-header-main">
                <div class="dm-file-avatar">
                  <i class="material-icons" :class="getFileIcon(selectedDownload?.filename)">{{ getFileIcon(selectedDownload?.filename) }}</i>
                </div>
                <div class="dm-file-info">
                  <h1 class="dm-filename" :title="selectedDownload?.filename">{{ selectedDownload?.filename || t('Details') }}</h1>
                  <div class="dm-meta-row">
                    <span class="dm-gid">{{ selectedDownload?.gid?.slice(0, 12) || '' }}</span>
                    <span class="dm-divider">â€¢</span>
                    <span class="dm-source-badge" :class="selectedDownload?.source === 'yt-dlp' ? 'yt' : 'http'">
                      <i class="material-icons">{{ selectedDownload?.source === 'yt-dlp' ? 'smart_display' : 'cloud_download' }}</i>
                      {{ selectedDownload?.source === 'yt-dlp' ? t('YouTube / yt-dlp') : t('HTTP / FTP') }}
                    </span>
                  </div>
                </div>
              </div>
              <div class="dm-header-actions">
                <span class="dm-status-pill" :class="selectedDownload?.status">
                  <i class="material-icons">{{ getStatusIcon(selectedDownload?.status) }}</i>
                  <span>{{ capitalizeStatus(selectedDownload?.status) }}</span>
                </span>
                <button class="dm-icon-btn" @click="copyToClipboard(selectedDownload?.gid)" :title="t('Copy GID')">
                  <i class="material-icons">content_copy</i>
                </button>
                <button class="dm-icon-btn dm-close" @click="showDetailsModal = false" :aria-label="t('Close')">
                  <i class="material-icons">close</i>
                </button>
              </div>
            </header>

            <nav class="dm-tabs" role="tablist">
              <button role="tab" class="dm-tab" :class="{ active: activeDetailsTab === 'general' }" @click="activeDetailsTab = 'general'">
                <i class="material-icons">dashboard</i><span>{{ t('General') }}</span>
              </button>
              <button role="tab" class="dm-tab" :class="{ active: activeDetailsTab === 'files' }" @click="activeDetailsTab = 'files'">
                <i class="material-icons">folder_open</i><span>{{ t('Files') }}</span>
              </button>
              <button role="tab" class="dm-tab" :class="{ active: activeDetailsTab === 'network' }" @click="activeDetailsTab = 'network'">
                <i class="material-icons">dns</i><span>{{ t('Network') }}</span>
              </button>
              <button role="tab" class="dm-tab" :class="{ active: activeDetailsTab === 'history' }" @click="activeDetailsTab = 'history'">
                <i class="material-icons">history</i><span>{{ t('History') }}</span>
              </button>
            </nav>

            <div class="dm-body">
              <section v-if="activeDetailsTab === 'general'" class="dm-panel" role="tabpanel">
                <div class="dm-section">
                  <h3 class="dm-section-title">{{ t('Transfer') }}</h3>
                  <table class="dm-table">
                    <tbody>
                      <tr>
                        <th>{{ t('Status') }}</th>
                        <td>
                          <span class="dm-status-badge" :class="selectedDownload?.status">
                            <i class="material-icons">{{ getStatusIcon(selectedDownload?.status) }}</i>
                            <span>{{ capitalizeStatus(selectedDownload?.status) }}</span>
                          </span>
                        </td>
                      </tr>
                      <tr>
                        <th>{{ t('Progress') }}</th>
                        <td class="dm-progress-cell">
                          <div class="dm-progress-wrap">
                            <div class="dm-progress-track" role="progressbar" :aria-valuenow="selectedDownload?.progress || 0" aria-valuemin="0" aria-valuemax="100">
                              <div class="dm-progress-fill" :style="{ width: (selectedDownload?.progress || 0) + '%' }"></div>
                            </div>
                            <span class="dm-progress-pct">{{ selectedDownload?.progress || 0 }}%</span>
                          </div>
                        </td>
                      </tr>
                      <tr>
                        <th>{{ t('Size') }}</th>
                        <td>{{ formatFileSize(Number(selectedDownloadDetails?.totalLength) || 0) }}</td>
                      </tr>
                      <tr v-if="selectedDownload?.speed && selectedDownload?.status === 'active'">
                        <th>{{ t('Speed') }}</th>
                        <td class="dm-speed-cell">{{ selectedDownload?.speed }}</td>
                      </tr>
                      <tr v-if="selectedDownload?.status === 'active'">
                        <th>{{ t('ETA') }}</th>
                        <td>{{ formatRemainingTime(selectedDownload?.remainingSeconds) }}</td>
                      </tr>
                    </tbody>
                  </table>
                </div>
                <div class="dm-section">
                  <h3 class="dm-section-title">{{ t('Configuration') }}</h3>
                  <table class="dm-table">
                    <tbody>
                      <tr>
                        <th>{{ t('Connections') }}</th>
                        <td>
                          <span class="dm-chip"><i class="material-icons">layers</i>
                            {{ selectedDownload?.connections ?? 1 }}
                          </span>
                        </td>
                      </tr>
                      <tr>
                        <th>{{ t('Active Connections') }}</th>
                        <td>{{ selectedDownload?.activeConnections || 1 }}</td>
                      </tr>
                      <tr>
                        <th>{{ t('Priority') }}</th>
                        <td><span class="dm-priority normal">{{ t('Normal') }}</span></td>
                      </tr>
                      <tr>
                        <th>{{ t('Source') }}</th>
                        <td>
                          <span class="dm-source-tag" :class="selectedDownload?.source === 'yt-dlp' ? 'yt' : 'http'">
                            {{ selectedDownload?.source === 'yt-dlp' ? t('YouTube (yt-dlp)') : t('HTTP / FTP') }}
                          </span>
                        </td>
                      </tr>
                      <tr>
                        <th>{{ t('Proxy') }}</th>
                        <td>
                          <span class="dm-source-tag" :class="{ direct: !selectedDownload?.proxy }">
                            {{ selectedDownload?.proxy || t('Direct') }}
                          </span>
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </section>

              <section v-if="activeDetailsTab === 'files'" class="dm-panel" role="tabpanel">
                <div v-if="selectedDownloadDetails?.files?.length" class="dm-file-table-wrapper">
                  <table class="dm-file-table">
                    <thead>
                      <tr>
                        <th>{{ t('File') }}</th>
                        <th>{{ t('Size') }}</th>
                        <th>{{ t('Progress') }}</th>
                        <th>{{ t('Path') }}</th>
                        <th></th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr v-for="(file, i) in selectedDownloadDetails.files" :key="i">
                        <td class="dm-file-name-cell">
                          <i class="material-icons" :class="getFileIcon(file.path)">{{ getFileIcon(file.path) }}</i>
                          <span class="dm-file-name">{{ file.path?.split(/[\\/]/).pop() || file.path }}</span>
                        </td>
                        <td>{{ formatFileSize(Number(file.length) || 0) }}</td>
                        <td class="dm-file-progress-cell">
                          <div v-if="file.length && file.completedLength" class="dm-mini-progress">
                            <div class="dm-mini-track">
                              <div class="dm-mini-fill" :style="{ width: ((Number(file.completedLength) / Number(file.length)) * 100) + '%' }"></div>
                            </div>
                            <span>{{ ((Number(file.completedLength) / Number(file.length)) * 100).toFixed(1) }}%</span>
                          </div>
                          <span v-else>â€”</span>
                        </td>
                        <td class="dm-path-cell" :title="file.path">{{ file.path }}</td>
                        <td>
                          <button class="dm-icon-btn" @click="copyToClipboard(file.path)" :title="t('Copy path')">
                            <i class="material-icons">content_copy</i>
                          </button>
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </div>
                <div v-else class="dm-empty-state">
                  <i class="material-icons">folder_open</i>
                  <p>{{ t('No file information available') }}</p>
                  <span class="dm-hint">{{ t('Files appear after download starts') }}</span>
                </div>
              </section>

              <section v-if="activeDetailsTab === 'network'" class="dm-panel" role="tabpanel">
                <div class="dm-section">
                  <h3 class="dm-section-title">{{ t('Connection') }}</h3>
                  <table class="dm-table">
                    <tbody>
                      <tr>
                        <th>{{ t('URL') }}</th>
                        <td class="dm-url-cell">
                          <span class="dm-url-text">{{ selectedDownload?.url || selectedDownload?.uri || 'â€”' }}</span>
                          <button v-if="selectedDownload?.url || selectedDownload?.uri" class="dm-icon-btn" @click="copyToClipboard(selectedDownload?.url || selectedDownload?.uri)" :title="t('Copy URL')">
                            <i class="material-icons">content_copy</i>
                          </button>
                        </td>
                      </tr>
                      <tr>
                        <th>{{ t('Folder') }}</th>
                        <td class="dm-path-cell" :title="getDownloadFolder()">
                          {{ getDownloadFolder() }}
                          <button class="dm-icon-btn" @click="copyToClipboard(getDownloadFolder())" :title="t('Copy path')">
                            <i class="material-icons">content_copy</i>
                          </button>
                        </td>
                      </tr>
                      <tr v-if="selectedDownload?.source === 'yt-dlp'">
                        <th>{{ t('Format') }}</th>
                        <td><span class="dm-chip yt"><i class="material-icons">smart_display</i> yt-dlp</span></td>
                      </tr>
                    </tbody>
                  </table>
                </div>
                <div v-if="selectedDownloadDetails?.source?.length" class="dm-section">
                  <h3 class="dm-section-title">{{ t('Source URLs') }}</h3>
                  <ul class="dm-source-list">
                    <li v-for="(src, i) in selectedDownloadDetails.source" :key="i">
                      <span class="dm-source-item">{{ src }}</span>
                      <button class="dm-icon-btn sm" @click="copyToClipboard(src)" :title="t('Copy')"><i class="material-icons">content_copy</i></button>
                    </li>
                  </ul>
                </div>

                <div class="dm-section">
                  <h3 class="dm-section-title">{{ t('Proxy') }}</h3>
                  <ProxyField v-model="detailProxy" />
                  <div class="dm-proxy-actions">
                    <button class="dm-proxy-save" @click="applyDetailProxy" :disabled="proxyApplying">
                      <i v-if="proxyApplying" class="material-icons nd-spinner">hourglass_top</i>
                      <i v-else class="material-icons">save</i>
                      {{ proxyApplying ? t('Saving...') : t('Save') }}
                    </button>
                    <span v-if="proxySaveMessage" class="dm-proxy-message" :class="{ ok: proxySaveOk }">{{ proxySaveMessage }}</span>
                  </div>
                </div>
              </section>

              <section v-if="activeDetailsTab === 'history'" class="dm-panel" role="tabpanel">
                <div class="dm-section">
                  <h3 class="dm-section-title">{{ t('Timeline') }}</h3>
                  <table class="dm-table">
                    <tbody>
                      <tr>
                        <th>{{ t('Added') }}</th>
                        <td>{{ formatDateTime(selectedDownload?.added_at) }}</td>
                      </tr>
                      <tr v-if="selectedDownloadDetails?.startTime">
                        <th>{{ t('Started') }}</th>
                        <td>{{ formatDateTime(selectedDownloadDetails?.startTime) }}</td>
                      </tr>
                      <tr v-if="selectedDownloadDetails?.completionTime">
                        <th>{{ t('Completed') }}</th>
                        <td>{{ formatDateTime(selectedDownloadDetails?.completionTime) }}</td>
                      </tr>
                    </tbody>
                  </table>
                </div>
                <div v-if="selectedDownloadDetails?.errorMessage" class="dm-section dm-error-section">
                  <h3 class="dm-section-title"><i class="material-icons">error_outline</i> {{ t('Error') }}</h3>
                  <div class="dm-error-box">
                    <pre>{{ selectedDownloadDetails.errorMessage }}</pre>
                    <button class="dm-btn-secondary" @click="copyToClipboard(selectedDownloadDetails.errorMessage)">
                      <i class="material-icons">content_copy</i> {{ t('Copy error') }}
                    </button>
                  </div>
                </div>
              </section>
            </div>

            <footer class="dm-footer">
              <button class="dm-btn" @click="showDetailsModal = false">
                <i class="material-icons">check</i> {{ t('Close') }}
              </button>
            </footer>
          </div>
        </transition>
      </div>
    </transition>

    <!-- Scroll-to-top button -->
    <button
      v-if="showScrollTop"
      @click="scrollToTop"
      class="scroll-top-btn"
      :title="t('Back to top')"
    >
      <i class="material-icons">keyboard_arrow_up</i>
    </button>
  </div>
</template>

<script setup lang="ts">
// -- Imports --
import { ref, inject, computed, onMounted, onUnmounted, nextTick, watch, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { open } from '@tauri-apps/plugin-dialog'
import ScheduleModal, { type ScheduleResult } from '../components/ScheduleModal.vue'
import SchedulePicker from '../components/SchedulePicker.vue'
import ProxyUrlCheckBadge from '../components/ProxyUrlCheckBadge.vue'
import ProxyField from '../components/ProxyField.vue'
import { emptyProxyForm, parseProxyString, serializeProxy, type ProxyFormValue } from '../utils/proxy'
import { useDragReorder, SORTABLE_BASE } from '../composables/useDragReorder'
import DragItem from '../components/DragItem.vue'
import CustomDropdown from '../components/CustomDropdown.vue'
import Sortable from 'sortablejs'
import type { SortableEvent } from 'sortablejs'
import { useGroups, type DownloadGroup } from '../composables/useGroups'
import { formatFileSize, formatDateTime, formatRemainingTime, parseDownloadSize, parseFormattedSize } from '../utils/format'
import DownloadListCard from '../components/DownloadListCard.vue'
import DownloadListCardCompact from '../components/DownloadListCardCompact.vue'
import ConnectionSelector from '../components/ConnectionSelector.vue'
import ErrorDetailsDialog from '../components/ErrorDetailsDialog.vue'
import BatchUrlDialog from '../components/BatchUrlDialog.vue'
import HashVerifyDialog from '../components/HashVerifyDialog.vue'
import CategoryPicker from '../components/CategoryPicker.vue'
import { loadDownloadCategories } from '../composables/useDownloadCategories'
import { resolveCategoryDir, type DownloadCategoryMode } from '../utils/downloadCategories'
import type { Download } from '../types/download'
import type { DownloadDetails } from '../types/settings'
import { useI18n } from '../i18n'

// -- Injection --
const notify = inject<(msg: string, type: 'success' | 'error' | 'info') => void>('notify', (_msg, _type) => console.log(_msg))
const searchQuery = inject<Ref<string>>('searchQuery', ref(''))
const rememberRequestedConnections = inject<(url: string, connections: number) => void>('rememberRequestedConnections', () => {})
const { t } = useI18n()

// -- Props & Emits --
const props = defineProps<{
  downloadFilter?: string
  downloads?: Download[]
}>()
const emit = defineEmits(['refresh', 'fast-refresh', 'cancel', 'remove'])

// -- Core State --
const downloads = computed(() => props.downloads || [])

// Check if entered URL already exists in downloads
const duplicateDownload = computed(() => {
  if (!newUrl.value.trim()) return null
  const normalizedUrl = newUrl.value.trim().toLowerCase()
  return downloads.value.find((d) =>
    d.url && d.url.toLowerCase() === normalizedUrl
  ) || null
})
const {
  groups,
  reloadGroups,
  saveGroups,
  createGroup,
  updateGroup,
  deleteGroup: removeGroup,
  addMember,
  removeMember,
  applyMemberLimit,
  applyGroupLimits,
  resetGroupDownloadLimit,
} = useGroups()
const viewMode = ref(localStorage.getItem('moonhunt_view_mode') || 'list')

// Sync groups from localStorage when changed externally (e.g., context menu "Add to Group")
function onGroupsChanged() {
  reloadGroups()
  remountSortablesIfGrabbed()
}
const downloadFilter = ref(props.downloadFilter || 'all')
const downloadFilterIndex = computed(() => {
  if (downloadFilter.value === 'downloading') return 1
  if (downloadFilter.value === 'completed') return 2
  return 0
})

interface DownloadPreferences {
  sort: string
  sortAscending: boolean
}

function initialSortAscending(): boolean {
  const storedAsc = localStorage.getItem('moonhunt_sort_asc')
  return storedAsc !== null ? storedAsc !== 'false' : false
}

const sortOption = ref(localStorage.getItem('moonhunt_sort') || 'added')
const sortAscending = ref<boolean>(initialSortAscending())
const sortOptions = ['added', 'name', 'size', 'completed', 'manual']
const sortLabels: Record<string, string> = {
  added: 'Date Added', name: 'Name', size: 'Size', completed: 'Date Completed', manual: 'Manual',
}
// Custom drag-reorder positions: ordered sequence of GIDs.
// Using a GID array instead of an index map avoids stale indices
// when downloads are added/removed/completed between reorders.
const customOrder = ref<string[]>([])
// Manual top-level layout for groups + ungrouped downloads. Values are either
// `g:<groupId>` or `d:<gid>` tokens, so manually dragged downloads can sit
// above groups instead of always being rendered under every group.
const manualLayout = ref<string[]>([])
const sortMenuOpen = ref(false)
const sortBtnRef = ref<HTMLElement | null>(null)
const stickySortBtnRef = ref<HTMLElement | null>(null)
let sortPreferencesLoaded = false

async function loadSortPreferences() {
  try {
    const saved = await invoke<DownloadPreferences | null>('get_download_preferences')
    if (!saved || sortPreferencesLoaded) return
    sortOption.value = saved.sort || sortOption.value
    sortAscending.value = saved.sortAscending
    sortPreferencesLoaded = true
    localStorage.setItem('moonhunt_sort', sortOption.value)
    localStorage.setItem('moonhunt_sort_asc', String(sortAscending.value))
  } catch {
    // Outside Tauri the backend is unavailable; keep localStorage defaults.
  }
}

function persistSortPreferences() {
  sortPreferencesLoaded = true
  localStorage.setItem('moonhunt_sort', sortOption.value)
  localStorage.setItem('moonhunt_sort_asc', String(sortAscending.value))
  try {
    void invoke('save_download_preferences', {
      preferences: {
        sort: sortOption.value,
        sortAscending: sortAscending.value,
      },
    }).catch(() => {})
  } catch {
    // Running outside Tauri; localStorage still keeps the preference.
  }
}

function setSortOption(opt: string) {
  sortOption.value = opt
  sortMenuOpen.value = false
  persistSortPreferences()
}

function setSortAscending(ascending: boolean) {
  sortAscending.value = ascending
  persistSortPreferences()
}

// -- Drag reorder (SortableJS, whole-card grab when enabled) --
// forceFallback + settle animation + data reorder onUpdate.
// Difference: no drag-handle rail — grab mode makes the entire card the drag surface.

function markManualOrder(gids: string[]) {
  customOrder.value = gids
  sortOption.value = 'manual'
  persistSortPreferences()
}

/** Extract ordered GIDs from visible .drag-item children of a container. */
function gidsFromDom(container: HTMLElement): string[] {
  return Array.from(container.children)
    .filter((el) => el.classList.contains('drag-item'))
    .map((el) => (el as HTMLElement).getAttribute('data-gid'))
    .filter((g): g is string => !!g)
}

function groupToken(id: string): string {
  return `g:${id}`
}

function downloadToken(gid: string): string {
  return `d:${gid}`
}

function parseManualToken(token: string): { kind: 'group'; id: string } | { kind: 'download'; gid: string } | null {
  if (token.startsWith('g:')) return { kind: 'group', id: token.slice(2) }
  if (token.startsWith('d:')) return { kind: 'download', gid: token.slice(2) }
  return null
}

/** Read the top-level order of groups and ungrouped cards straight from the DOM. */
function topLevelOrderFromDom(container: HTMLElement): string[] {
  return Array.from(container.children)
    .map((el) => {
      if (el.classList.contains('group-container')) {
        const id = (el as HTMLElement).id.replace(/^group-/, '')
        return id ? groupToken(id) : null
      }
      if (el.classList.contains('drag-item')) {
        const gid = (el as HTMLElement).getAttribute('data-gid')
        return gid ? downloadToken(gid) : null
      }
      return null
    })
    .filter((x): x is string => !!x)
}

/** Keep the manual layout current while dropping stale/grouped entries. */
function normalizeManualLayout(): string[] {
  const groupTokens = new Set(groups.value.map((g) => groupToken(g.id)))
  const ungroupedTokens = new Set(
    sortedDownloads.value
      .filter((d) => !groups.value.some((g) => g.memberGids.includes(d.gid)))
      .map((d) => downloadToken(d.gid)),
  )
  const seen = new Set<string>()
  const layout: string[] = []

  for (const token of manualLayout.value) {
    const parsed = parseManualToken(token)
    if (!parsed) continue
    if (parsed.kind === 'group') {
      if (groupTokens.has(token) && !seen.has(token)) {
        layout.push(token)
        seen.add(token)
      }
    } else if (parsed.kind === 'download') {
      if (ungroupedTokens.has(token) && !seen.has(token)) {
        layout.push(token)
        seen.add(token)
      }
    }
  }

  for (const token of groupTokens) {
    if (!seen.has(token)) {
      layout.push(token)
      seen.add(token)
    }
  }
  for (const token of ungroupedTokens) {
    if (!seen.has(token)) {
      layout.push(token)
      seen.add(token)
    }
  }
  return layout
}

/**
 * Reorder ungrouped cards from current DOM order (top-level children .drag-item).
 * Safer than raw Sortable indices when groups share the same list.
 */
function reorderUngroupedFromDom(container: HTMLElement) {
  const ungroupedGids = gidsFromDom(container)
  const layout = topLevelOrderFromDom(container)
  if (layout.length) manualLayout.value = layout
  if (!ungroupedGids.length) return

  const ungroupedSet = new Set(ungroupedGids)
  const base =
    customOrder.value.length > 0
      ? [...customOrder.value]
      : sortedDownloads.value.map((d) => d.gid)
  const rest = base.filter((g) => !ungroupedSet.has(g))
  const missing = sortedDownloads.value
    .map((d) => d.gid)
    .filter((g) => !rest.includes(g) && !ungroupedSet.has(g))
  markManualOrder([...rest, ...missing, ...ungroupedGids])
}

function reorderWithinGroupFromDom(groupId: string, body: HTMLElement) {
  const orderedGids = gidsFromDom(body)
  if (!orderedGids.length) return
  // Preserve members hidden by downloadFilter — they're not in the DOM
  const group = groups.value.find((g) => g.id === groupId)
  const hiddenMembers = group
    ? group.memberGids.filter((gid) => !orderedGids.includes(gid))
    : []
  updateGroup(groupId, { memberGids: [...orderedGids, ...hiddenMembers] })
  const orderedSet = new Set(orderedGids)
  const base =
    customOrder.value.length > 0
      ? [...customOrder.value]
      : sortedDownloads.value.map((d) => d.gid)
  const without = base.filter((g) => !orderedSet.has(g))
  let insertAt = base.findIndex((g) => orderedSet.has(g))
  if (insertAt < 0) insertAt = without.length
  insertAt = Math.min(insertAt, without.length)
  without.splice(insertAt, 0, ...orderedGids)
  markManualOrder(without)
}

const {
  grabEnabled,
  toggleGrabEnabled,
  sorting,
  mountSortable,
  destroyAllSortables,
  startFloatingRectTracking,
  animateDropSettle,
  stopFloatingRectTracking,
} = useDragReorder({})

const listContainerRef = ref<HTMLElement | null>(null)
const compactContainerRef = ref<HTMLElement | null>(null)
const dragOverGroupId = ref<string | null>(null)
let pendingDropGroupId: string | null = null
// Bump to force Vue to rebuild the drag containers after a rejected move
// that Sortable has already removed from the DOM.
const listKey = ref(0)
const forceRerenderLists = () => { listKey.value++ }
let groupSortables: Sortable[] = []

function groupIdFromEl(el: Element | null): string | null {
  const group = el?.closest?.('.group-container') ?? null
  if (!group?.id?.startsWith('group-')) return null
  return group.id.slice('group-'.length)
}

function clearDragOverHighlights() {
  dragOverGroupId.value = null
  document.querySelectorAll('.group-container.drag-over').forEach((c) => c.classList.remove('drag-over'))
  document.querySelectorAll('.group-header').forEach((h) => h.classList.remove('drag-over'))
}

function resetDragOverHighlights() {
  pendingDropGroupId = null
  clearDragOverHighlights()
}

interface GroupDropMoveEvent {
  to: HTMLElement
  dragged: HTMLElement
  originalEvent?: Event | null
}

function findGroupDropTarget(originalEvent?: Event | null): string | null | undefined {
  if (!originalEvent || !('clientX' in originalEvent) || !('clientY' in originalEvent)) return undefined
  const x = (originalEvent as MouseEvent).clientX
  const y = (originalEvent as MouseEvent).clientY
  const elements = document.elementsFromPoint(x, y)
  for (const el of elements) {
    const id = groupIdFromEl(el)
    if (id) return id
  }
  return null
}

/** Shared onMove handler — turn the hovered group card into the drop area. */
function highlightGroupDropTarget(event: GroupDropMoveEvent): boolean {
  if (event.dragged?.classList?.contains('group-container')) {
    clearDragOverHighlights()
    return !event.to.classList.contains('group-downloads')
  }
  clearDragOverHighlights()
  const draggedGid = event.dragged?.getAttribute?.('data-gid')
  const draggedIsCompleted =
    !!draggedGid &&
    downloads.value.some((d) => d.gid === draggedGid && d.status === 'complete')

  const id = findGroupDropTarget(event.originalEvent) ?? groupIdFromEl(event.to)
  if (id && draggedIsCompleted) {
    // Completed downloads can't move into a group; don't even suggest it.
    pendingDropGroupId = null
    return false
  }
  if (id) {
    dragOverGroupId.value = id
    pendingDropGroupId = id
    document.querySelector(`#group-${id}`)?.classList.add('drag-over')
    document.querySelector(`#group-${id} > .group-header`)?.classList.add('drag-over')
  } else {
    pendingDropGroupId = null
  }
  return true
}

/**
 * Cross-list membership — committed only on drop (onEnd), not mid-hover.
 * Mid-drag onAdd/onRemove is what made groups expand/retract at the header seam.
 */
function commitCrossListDrop(event: SortableEvent) {
  const item = event.item
  const gid = item?.getAttribute?.('data-gid')
  if (!gid) return
  const fromGroupId = groupIdFromEl(event.from as HTMLElement)
  const originalEvent = (event as SortableEvent & { originalEvent?: Event | null }).originalEvent
  const pointerGroupId = findGroupDropTarget(originalEvent)
  const toGroupId =
    pointerGroupId ??
    groupIdFromEl(event.to as HTMLElement) ??
    // Only trust the hover target when the drop event has no usable pointer
    // coordinates (e.g. touch). A stale group highlight must not keep a card
    // trapped when it was actually dropped back on the top-level list.
    (pointerGroupId === undefined ? pendingDropGroupId ?? dragOverGroupId.value : null)
  if (event.from === event.to && !toGroupId) return
  if (fromGroupId === toGroupId) return

  // Remember where the card landed at the top level before Sortable strips it.
  if (!toGroupId) {
    const toEl = event.to as HTMLElement
    if (toEl.classList.contains('drag-list-container')) {
      const layout = topLevelOrderFromDom(toEl)
      if (layout.length) {
        manualLayout.value = layout
        sortOption.value = 'manual'
        persistSortPreferences()
      }
    }
  }

  // Strip Sortable's node so Vue re-render is the only copy; we own membership in the groups store.
  if (item.isConnected) item.remove()

  if (fromGroupId && fromGroupId !== toGroupId) {
    removeMemberFromGroup(fromGroupId, gid).catch((e) =>
      console.error('Failed to remove from group during cross-group drop:', e),
    )
  }
  if (toGroupId && toGroupId !== fromGroupId) {
    const dl = downloads.value.find((d) => d.gid === gid)
    if (dl?.status === 'complete') {
      // Sortable has already removed the DOM node; force Vue to rebuild it
      // from the unchanged list state so the card comes back in place.
      notify(t('Cannot add completed download to group'), 'error')
      forceRerenderLists()
      return
    }
    addToGroup(toGroupId, gid).catch((e) =>
      console.error('Failed to add to group during cross-group drop:', e),
    )
  }
}

function destroyGroupSortables() {
  groupSortables.forEach((s) => {
    try {
      s.destroy()
    } catch {
      /* already destroyed */
    }
  })
  groupSortables = []
}

function destroyAllSortablesAndListeners() {
  document.removeEventListener('mouseup', clearDragOverHighlights)
  document.removeEventListener('pointerup', clearDragOverHighlights)
  resetDragOverHighlights()
  destroyGroupSortables()
  destroyAllSortables()
}

function mountGroupSortables() {
  if (!grabEnabled.value) return

  const groupBodies = document.querySelectorAll(
    '.group-downloads.drag-list-container',
  ) as NodeListOf<HTMLElement>

  groupBodies.forEach((body) => {
    const groupId = groupIdFromEl(body)
    // Use raw Sortable with the shared base options
    // Nested body lists share group name so cards can move in/out.
    const s = Sortable.create(body, {
      ...SORTABLE_BASE,
      draggable: '.drag-item',
      group: { name: 'download-items', pull: true, put: true },
      onStart: (event: SortableEvent) => {
        resetDragOverHighlights()
        sorting.value = true
        startFloatingRectTracking()
      },
      onUpdate: (event: SortableEvent) => {
        if (
          event.oldIndex === undefined ||
          event.newIndex === undefined ||
          event.oldIndex === event.newIndex
        ) {
          return
        }
        if (!groupId) return
        // Mutate order from indices; read DOM gids after splice for stability
        reorderWithinGroupFromDom(groupId, body)
      },
      onMove: (event: GroupDropMoveEvent) => highlightGroupDropTarget(event),
      onEnd: async (event: SortableEvent) => {
        stopFloatingRectTracking()
        commitCrossListDrop(event)
        resetDragOverHighlights()
        await nextTick()
        await animateDropSettle(event)
        window.setTimeout(() => {
          sorting.value = false
        }, 0)
      },
    })
    groupSortables.push(s)
  })
}

function mountTopLevelSortables() {
  if (!grabEnabled.value) return
  const containers = [listContainerRef.value, compactContainerRef.value].filter(
    Boolean,
  ) as HTMLElement[]

  containers.forEach((el) => {
    // Mount one Sortable on the list; same for the top-level
    // container. Whole card / group shell is draggable — no handle.
    mountSortable(el, {
      draggable: '.group-container, .drag-item',
      group: { name: 'download-items', pull: true, put: true },
      onStart: () => resetDragOverHighlights(),
      onUpdate: (event: SortableEvent) => {
        if (
          event.oldIndex === undefined ||
          event.newIndex === undefined ||
          event.oldIndex === event.newIndex
        ) {
          return
        }
        const layout = topLevelOrderFromDom(el)
        if (layout.length) manualLayout.value = layout
        const item = event.item
        if (item.classList.contains('group-container')) {
          // Rebuild group order from DOM; we own the groups store
          const newOrder = Array.from(el.children)
            .filter((c) => c.classList.contains('group-container'))
            .map((c) => c.id.replace(/^group-/, ''))
            .filter(Boolean)
          const byId = new Map(groups.value.map((g) => [g.id, g]))
          const next = newOrder
            .map((id) => byId.get(id))
            .filter((g): g is NonNullable<typeof g> => !!g)
          for (const g of groups.value) {
            if (!newOrder.includes(g.id)) next.push(g)
          }
          groups.value = next
          saveGroups()
        } else if (item.classList.contains('drag-item')) {
          reorderUngroupedFromDom(el)
        }
      },
      onMove: (event: GroupDropMoveEvent) => highlightGroupDropTarget(event),
      onEnd: (event: SortableEvent) => {
        commitCrossListDrop(event)
        resetDragOverHighlights()
      },
    })
  })
}

/** Destroy then create on mount / page change. */
function mountAllSortables() {
  destroyAllSortablesAndListeners()
  if (!grabEnabled.value) return
  mountTopLevelSortables()
  mountGroupSortables()
  document.addEventListener('mouseup', clearDragOverHighlights)
  document.addEventListener('pointerup', clearDragOverHighlights)
}

/** Remount sortables after group DOM changes while grab is on. */
function remountSortablesIfGrabbed() {
  if (grabEnabled.value) {
    nextTick(mountAllSortables)
  }
}

watch(grabEnabled, (enabled) => {
  if (enabled) {
    nextTick(mountAllSortables)
  } else {
    destroyAllSortablesAndListeners()
  }
})

// Remount Sortable after page/view transition
watch(viewMode, () => {
  if (grabEnabled.value) {
    nextTick(mountAllSortables)
  }
})

// -- Filtered & sorted downloads --
const filteredDownloads = computed(() => {
  let list = downloads.value
  if (downloadFilter.value === 'downloading') {
    list = list.filter((d) => ['active', 'waiting', 'paused'].includes(d.status))
  } else if (downloadFilter.value === 'completed') {
    list = list.filter((d) => ['complete', 'error'].includes(d.status))
  }
  return list
})

const sortedDownloads = computed(() => {
  let list = [...filteredDownloads.value]

  if (searchQuery.value.trim()) {
    const q = searchQuery.value.toLowerCase()
    list = list.filter((d) =>
      d.filename.toLowerCase().includes(q) ||
      d.url.toLowerCase().includes(q)
    )
  }

  const sort = sortOption.value
  const asc = sortAscending.value

  if (sort === 'name') {
    list.sort((a, b) => (a.filename || '').localeCompare(b.filename || ''))
  } else if (sort === 'size') {
    list.sort((a, b) => {
      const aTotal = parseDownloadSize(a.size || '')
      const bTotal = parseDownloadSize(b.size || '')
      return aTotal - bTotal
    })
  } else if (sort === 'completed') {
    list.sort((a, b) => (a.added_at || 0) - (b.added_at || 0))
  } else if (sort === 'manual' && customOrder.value.length > 0) {
    list.sort((a, b) => {
      const aPos = customOrder.value.indexOf(a.gid)
      const bPos = customOrder.value.indexOf(b.gid)
      if (aPos !== -1 && bPos !== -1) return aPos - bPos
      if (aPos !== -1) return -1
      if (bPos !== -1) return 1
      return 0
    })
  } else {
    list.sort((a, b) => (a.added_at || 0) - (b.added_at || 0))
  }

  if (!asc && sort !== 'manual') list.reverse()

  return list
})

// Grouped downloads
type GroupEntry = {
  group: DownloadGroup
  downloads: Array<{ download: Download; flatIndex: number }>
  dateAdded: number
}
type GroupedItem =
  | { type: 'group'; group: DownloadGroup; downloads: GroupEntry['downloads']; dateAdded: number }
  | { type: 'download'; download: Download; flatIndex: number }
const groupedDownloads = computed<GroupedItem[]>(() => {
  const items: GroupedItem[] = []
  const usedGids = new Set<string>()
  const usedGroups = new Set<string>()
  const sort = sortOption.value

  const memberGidSet = new Set(groups.value.flatMap((g) => g.memberGids))

  const groupEntries: GroupEntry[] = groups.value.map((group) => {
    const groupDownloads = sortedDownloads.value
      .filter((d) => group.memberGids.includes(d.gid))
      .map((d) => ({
        download: d,
        flatIndex: sortedDownloads.value.indexOf(d),
      }))
    const earliestMemberAdded = groupDownloads.reduce(
      (min, d) => Math.min(min, d.download.added_at || Number.POSITIVE_INFINITY),
      Number.POSITIVE_INFINITY,
    )
    const dateAdded =
      group.added_at && group.added_at > 0
        ? group.added_at
        : Number.isFinite(earliestMemberAdded)
          ? earliestMemberAdded
          : 0
    return { group, downloads: groupDownloads, dateAdded }
  }).filter((entry) => entry.downloads.length > 0 || downloadFilter.value === 'all')

  const groupItems: GroupedItem[] = groupEntries.map((entry) => ({ type: 'group' as const, ...entry }))
  const ungroupedDownloads: GroupedItem[] = sortedDownloads.value
    .filter((d) => !memberGidSet.has(d.gid))
    .map((download, flatIndex) => ({ type: 'download' as const, download, flatIndex }))

  if (sort === 'manual') {
    const groupBy = new Map(groupEntries.map((entry) => [entry.group.id, entry]))
    const downloadByGid = new Map(sortedDownloads.value.map((d) => [d.gid, d]))
    for (const token of normalizeManualLayout()) {
      const parsed = parseManualToken(token)
      if (parsed?.kind === 'group') {
        const entry = groupBy.get(parsed.id)
        if (!entry || usedGroups.has(parsed.id)) continue
        usedGroups.add(parsed.id)
        items.push({ type: 'group', ...entry })
        for (const gd of entry.downloads) usedGids.add(gd.download.gid)
      } else if (parsed?.kind === 'download') {
        const download = downloadByGid.get(parsed.gid)
        if (!download || usedGids.has(parsed.gid)) continue
        usedGids.add(parsed.gid)
        items.push({
          type: 'download',
          download,
          flatIndex: sortedDownloads.value.indexOf(download),
        })
      }
    }
    for (const entry of groupEntries) {
      if (usedGroups.has(entry.group.id)) continue
      items.push({ type: 'group', ...entry })
      usedGroups.add(entry.group.id)
    }
    for (const download of ungroupedDownloads) {
      if (download.type !== 'download' || usedGids.has(download.download.gid)) continue
      items.push(download)
    }
    return items
  }

  const itemName = (item: GroupedItem): string =>
    item.type === 'group' ? item.group.name : item.download.filename || ''
  const itemValue = (item: GroupedItem, key: 'added' | 'size' | 'completed'): number => {
    if (item.type === 'group') {
      if (key === 'size') {
        return item.downloads.reduce((sum, d) => sum + parseDownloadSize(d.download.size || ''), 0)
      }
      return item.dateAdded
    }
    if (key === 'size') return parseDownloadSize(item.download.size || '')
    return item.download.added_at || 0
  }

  items.push(...groupItems, ...ungroupedDownloads)
  if (sort === 'name') {
    items.sort((a, b) => itemName(a).localeCompare(itemName(b)))
  } else {
    const key = sort === 'size' ? 'size' : sort === 'completed' ? 'completed' : 'added'
    items.sort((a, b) => itemValue(a, key) - itemValue(b, key))
  }
  if (!sortAscending.value) items.reverse()
  return items
})

const hasNoUngroupedDownloads = computed(() => {
  return groupedDownloads.value.length > 0 && groupedDownloads.value.every(item => item.type === 'group')
})

function parseGroupSize(dl: Array<{ download: Download }>): string {
  let completed = 0
  let total = 0
  for (const d of dl) {
    const sizeStr = d.download.size || '0 KB / 0 KB'
    const parts = sizeStr.split('/')
    completed += parts.length === 2 ? parseFormattedSize(parts[0]) : parseDownloadSize(sizeStr)
    total += parseDownloadSize(sizeStr)
  }
  return `${formatFileSize(completed)} / ${formatFileSize(total)}`
}

function parseGroupProgress(dl: Array<{ download: Download }>): number {
  if (!dl.length) return 0
  let completed = 0
  let total = 0
  for (const d of dl) {
    const sizeStr = d.download.size || '0 KB / 0 KB'
    const parts = sizeStr.split('/')
    completed += parts.length === 2 ? parseFormattedSize(parts[0]) : parseDownloadSize(sizeStr)
    total += parseDownloadSize(sizeStr)
  }
  return total > 0 ? Math.min(100, Math.round((completed / total) * 100)) : 0
}

function formatGroupDate(value?: number): string {
  if (!value) return '—'
  const d = new Date(value)
  return isNaN(d.getTime()) ? '—' : d.toLocaleDateString()
}

// -- View Mode --
function setViewMode(mode: string) {
  // Cancel any active drag before switching views â€” the drag state
  // (indices, ghost clone, snapshot rects) belongs to the old view's DOM.
  if (sorting.value) {
    toggleGrabEnabled()
  }
  viewMode.value = mode
  localStorage.setItem('moonhunt_view_mode', mode)
}

// -- Sticky toolbar on scroll --
const showStickyToolbar = ref(false)
let scrollHandler: (() => void) | null = null

onMounted(() => {
  scrollHandler = () => {
    const container = document.querySelector<HTMLElement>('.content-area') || document.documentElement
    const top = container.scrollTop ?? window.scrollY ?? 0
    showStickyToolbar.value = top > 160
    showScrollTop.value = top > 400
  }
  const container = document.querySelector<HTMLElement>('.content-area') || document.documentElement
  container.addEventListener('scroll', scrollHandler, { passive: true })
  window.addEventListener('scroll', scrollHandler, { passive: true })
  scrollHandler()
  window.addEventListener('moonhunt:groups-changed', onGroupsChanged)
  window.addEventListener('moonhunt:torrent-drop', torrentDropHandler)
  void loadSortPreferences()
  emit('refresh')
})

onUnmounted(() => {
  destroyAllSortablesAndListeners()
  if (scrollHandler) {
    const container = document.querySelector<HTMLElement>('.content-area') || document.documentElement
    container.removeEventListener('scroll', scrollHandler)
    window.removeEventListener('scroll', scrollHandler)
  }
  window.removeEventListener('moonhunt:groups-changed', onGroupsChanged)
  window.removeEventListener('moonhunt:torrent-drop', torrentDropHandler)
})

// Close sort menu on outside click
onMounted(() => {
  const handler = (e: MouseEvent) => {
    const clickedSortButton =
      sortBtnRef.value?.contains(e.target as Node) ||
      stickySortBtnRef.value?.contains(e.target as Node)
    if (!clickedSortButton && !(e.target as HTMLElement).closest('.sort-menu')) {
      sortMenuOpen.value = false
    }
  }
  document.addEventListener('click', handler)
  onUnmounted(() => document.removeEventListener('click', handler))

  const showDetailsHandler = (e: Event) => {
    const detail = (e as CustomEvent).detail
    if (detail) showDownloadDetails(detail as Download)
  }
  window.addEventListener('moonhunt:show-details', showDetailsHandler)
  onUnmounted(() => window.removeEventListener('moonhunt:show-details', showDetailsHandler))

  const hashHandler = (e: Event) => {
    const detail = (e as CustomEvent).detail
    if (detail) {
      hashFilePath.value = detail.filePath || ''
      hashFilename.value = detail.filename || ''
      showHashDialog.value = true
    }
  }
  window.addEventListener('moonhunt:verify-hash', hashHandler)
  onUnmounted(() => window.removeEventListener('moonhunt:verify-hash', hashHandler))

  const newDlHandler = () => { showAddDialog.value = true }
  const batchDlHandler = () => { showBatchDialog.value = true }
  window.addEventListener('moonhunt:new-download', newDlHandler)
  window.addEventListener('moonhunt:batch-download', batchDlHandler)
  // MainLayout dispatches moonhunt:torrent-drop when a .torrent is dropped
  // anywhere on the app window (not just inside this page's dialog).
  window.addEventListener('moonhunt:torrent-drop', torrentDropHandler)
  onUnmounted(() => {
    window.removeEventListener('moonhunt:new-download', newDlHandler)
    window.removeEventListener('moonhunt:batch-download', batchDlHandler)
    window.removeEventListener('moonhunt:torrent-drop', torrentDropHandler)
  })
})

// -- Data fetching --
// Downloads are provided by MainLayout via props â€” no internal fetch needed.
// Actions emit events to MainLayout to trigger refreshes.

// -- New Download Dialog --
const showAddDialog = ref(false)
// Outside-click close that only fires when the press AND release both happen
// on the overlay. Pressing inside the modal and releasing outside keeps the
// modal open (mousedown-outside + mouseup-outside required).
let outsidePressOnOverlay = false
function startOutsideClose() { outsidePressOnOverlay = true }
function cancelOutsideClose() { outsidePressOnOverlay = false }
function finishOutsideClose() {
  if (outsidePressOnOverlay) {
    outsidePressOnOverlay = false
    showAddDialog.value = false
  }
}
// Shared guard for the other inline modals (create/rename group, group downloads).
let closeOutsideArmed = false
function closeOutsidePress() { closeOutsideArmed = true }
function closeOutsideCancel() { closeOutsideArmed = false }
function closeOutsideRelease(target: 'create' | 'rename' | 'groupDl') {
  if (!closeOutsideArmed) return
  closeOutsideArmed = false
  if (target === 'create') showCreateGroupDialog.value = false
  else if (target === 'rename') renameGroupId.value = null
  else showGroupDownloads.value = false
}
const showBatchDialog = ref(false)
const showHashDialog = ref(false)
const hashFilePath = ref('')
const hashFilename = ref('')
const addDialogTab = ref<'url' | 'torrent'>('url')
const newUrl = ref('')
const newFilename = ref('')
const newPath = ref('')
const connections = ref(16)
const defaultDir = ref('')
const categoriesEnabled = ref(false)
const categoryMode = ref<DownloadCategoryMode>('formatted')
const selectedCategory = ref('')
const categoryError = ref('')
const isAdding = ref(false)
const allowDuplicate = ref(false)
const showAdvanced = ref(false)
const wasPastedFromClipboard = ref(false)
const downloadProxy = ref<ProxyFormValue>(emptyProxyForm())
// Torrent tab state
interface TorrentFileEntry {
  path: string
  name: string
  files: Array<{ path: string; size: number }>
  selected: Set<number>
  loading: boolean
  error: string
}
const torrentFileList = ref<TorrentFileEntry[]>([])
const torrentMagnet = ref('')
const isDraggingTorrent = ref(false)

const detectedHost = computed(() => {
  if (!newUrl.value) return ''
  try {
    return new URL(newUrl.value.trim()).hostname.replace(/^www\./, '')
  } catch {
    return ''
  }
})

watch(showAddDialog, async (open) => {
  if (open) {
    void loadDownloadCategories()
    void loadDefaultDir()
    allowDuplicate.value = false
  }
  if (open && !newUrl.value) {
    wasPastedFromClipboard.value = false
    showAdvanced.value = false
    try {
      const text = await invoke<string>('read_clipboard_text')
      const { detectResource } = await import('../utils/detectResource')
      if (text && detectResource(text.trim())) {
        const firstUrl = text.trim().split('\n').map(l => l.trim()).find(Boolean)
        if (firstUrl) {
          newUrl.value = firstUrl
          wasPastedFromClipboard.value = true
        }
      }
    } catch { /* clipboard read denied or empty — ignore */ }
  } else if (open) {
    showAdvanced.value = false
    wasPastedFromClipboard.value = false
  }
})

async function loadDefaultDir() {
  if (defaultDir.value) return defaultDir.value
  try {
    defaultDir.value = await invoke<string>('get_default_download_dir')
  } catch {
    defaultDir.value = ''
  }
  return defaultDir.value
}

const detectedFilename = computed(() => {
  if (!newUrl.value) return ''
  try {
    const u = new URL(newUrl.value.trim())
    const parts = u.pathname.split('/')
    const last = parts[parts.length - 1]
    return last ? decodeURIComponent(last) : ''
  } catch {
    return ''
  }
})

let lastAutoFilledFilename = ''
watch(newUrl, () => {
  if (!newUrl.value.trim()) {
    newFilename.value = ''
    lastAutoFilledFilename = ''
    return
  }
  const detected = detectedFilename.value
  if (!detected) return
  if (!newFilename.value || newFilename.value === lastAutoFilledFilename) {
    newFilename.value = detected
    lastAutoFilledFilename = detected
  }
})

// Schedule
const scheduleEnabled = ref(false)
const pendingSchedule = ref<ScheduleResult | null>(null)

async function selectDownloadFolder() {
  try {
    const selected = await open({ directory: true, multiple: false, defaultPath: newPath.value || 'C:\\Downloads', title: t('Select Download Folder') })
    if (selected) newPath.value = selected as string
  } catch (error) {
    notify(t('Failed to select folder') + ': ' + error, 'error')
  }
}

async function loadTorrentFilesFor(entry: TorrentFileEntry) {
  entry.loading = true
  entry.error = ''
  try {
    const files = await invoke<Array<{ path: string; size: number }>>('parse_torrent_files', { torrentPath: entry.path })
    entry.files = files || []
    // Default: all files selected
    entry.selected = new Set(entry.files.map((_, i) => i))
  } catch (e: unknown) {
    entry.error = t('Failed to read torrent contents') + ': ' + e
    entry.files = []
    entry.selected = new Set()
  } finally {
    entry.loading = false
    // The entry was pushed while the OS drag was still in progress; on
    // Windows the DOM patch for the card list can be deferred until the
    // drag gesture fully ends, which only happens once the pointer
    // re-enters the window — leaving the card stuck on "Reading
    // contents...". Forcing a fresh array reference re-runs the v-for
    // diff synchronously.
    torrentFileList.value = [...torrentFileList.value]
  }
}

function addTorrentFileEntry(path: string) {
  // Skip duplicates
  if (torrentFileList.value.some((t) => t.path === path)) return
  const entry: TorrentFileEntry = {
    path,
    name: path.split(/[\\/]/).pop() || path,
    files: [],
    selected: new Set(),
    loading: false,
    error: '',
  }
  // Tauri's tauri://drag-drop event fires while the OS drag is still in
  // progress, and the dropzone's own mouseenter is never triggered by it.
  // Write the entry synchronously (not via nextTick) so the card — and its
  // "Reading contents..." state — renders before the drag ends; otherwise the
  // parse can still complete but the template patch is deferred until the
  // next event loop flush, which on some Windows builds only happens once
  // the user re-enters the dropzone with a new drag.
  torrentFileList.value = [...torrentFileList.value, entry]
  loadTorrentFilesFor(entry).catch((err) => console.error('Failed to load torrent files:', err))
}

function removeTorrentFile(index: number) {
  torrentFileList.value.splice(index, 1)
}

function clearTorrentFiles() {
  torrentFileList.value = []
}

async function pickTorrentFiles() {
  try {
    const selected = await open({ directory: false, multiple: true, filters: [{ name: 'Torrent', extensions: ['torrent'] }], defaultPath: 'C:\\Downloads', title: t('Select Torrent Files') })
    if (selected) {
      const paths = Array.isArray(selected) ? selected : [selected]
      for (const p of paths) {
        addTorrentFileEntry(p as string)
      }
    }
  } catch (error) {
    notify(t('Failed to select torrent files') + ': ' + error, 'error')
  }
}

const selectedTorrentFilesOf = (t: TorrentFileEntry) =>
  t.files.filter((_, i) => t.selected.has(i))

/**
 * Global torrent drop handler (dispatched by MainLayout when a .torrent is
 * dropped anywhere on the app window). Opens the New Download dialog on the
 * Torrent tab and queues the dropped files into it.
 */
function torrentDropHandler(e: Event) {
  const paths: string[] = (e as CustomEvent).detail?.paths || []
  if (!paths.length) return
  addDialogTab.value = 'torrent'
  showAddDialog.value = true
  for (const p of paths) {
    addTorrentFileEntry(p)
  }
}

function toggleTorrentFileIn(ti: number, fi: number) {
  const t = torrentFileList.value[ti]
  if (!t) return
  const next = new Set(t.selected)
  if (next.has(fi)) {
    next.delete(fi)
  } else {
    next.add(fi)
  }
  t.selected = next
}

function toggleAllTorrentFilesIn(ti: number) {
  const t = torrentFileList.value[ti]
  if (!t) return
  if (t.selected.size === t.files.length) {
    t.selected = new Set()
  } else {
    t.selected = new Set(t.files.map((_, i) => i))
  }
}

function formatTorrentTotal(files: Array<{ path: string; size: number }>): string {
  const total = files.reduce((sum, f) => sum + (Number(f.size) || 0), 0)
  return formatFileSize(total)
}

// Tauri intercepts OS-level file drags and emits them as window events
// (tauri://drag-enter / drag-over / drag-drop / drag-leave) instead of
// forwarding them to the DOM. We listen for those and check whether the
// pointer is over the dropzone to drive the highlight + selection.
const dropzoneEl = ref<HTMLElement | null>(null)
let dragDropUnlisten: Array<() => void> = []
let dragPointerInside = false

function isPointerOverDropzone(x: number, y: number): boolean {
  const el = dropzoneEl.value
  if (!el) return false
  const rect = el.getBoundingClientRect()
  return x >= rect.left && x <= rect.right && y >= rect.top && y <= rect.bottom
}

async function setupTauriDragDrop() {
  const { listen } = await import('@tauri-apps/api/event')
  // Use the internal event names Tauri emits (prefixed with tauri://)
  const events = [
    listen<{ paths?: string[]; position: { x: number; y: number } }>('tauri://drag-enter', (e) => {
      const pos = e.payload?.position
      dragPointerInside = pos ? isPointerOverDropzone(pos.x, pos.y) : false
      if (dragPointerInside) isDraggingTorrent.value = true
    }),
    listen<{ paths?: string[]; position: { x: number; y: number } }>('tauri://drag-over', (e) => {
      const pos = e.payload?.position
      dragPointerInside = pos ? isPointerOverDropzone(pos.x, pos.y) : false
      isDraggingTorrent.value = dragPointerInside
    }),
    listen<{ paths?: string[]; position: { x: number; y: number } }>('tauri://drag-drop', (e) => {
      isDraggingTorrent.value = false
      dragPointerInside = false
      const paths = e.payload?.paths || []
      // Accept drops anywhere on the modal (user preference) — any dropped
      // .torrent file lands in the torrent list.
      const torrents = paths.filter((p) => p.toLowerCase().endsWith('.torrent'))
      const skipped = paths.length - torrents.length
      for (const p of torrents) {
        addTorrentFileEntry(p)
      }
      if (skipped > 0) {
        notify(`${t('Skipped')} ${skipped} ${skipped === 1 ? t('non-torrent file') : t('non-torrent files')}`, 'error')
      }
    }),
    listen('tauri://drag-leave', () => {
      isDraggingTorrent.value = false
      dragPointerInside = false
    }),
  ]
  dragDropUnlisten = await Promise.all(events)
}

onMounted(() => {
  setupTauriDragDrop().catch((err) => console.error('Failed to setup drag-drop listener:', err))
})

onUnmounted(() => {
  dragDropUnlisten.forEach((fn) => { try { fn() } catch { /* */ } })
  dragDropUnlisten = []
})

function clearTorrentFile() {
  clearTorrentFiles()
}

async function addTorrent() {
  if (isAdding.value) return
  if (torrentFileList.value.length === 0 && !torrentMagnet.value.trim()) {
    notify(t('Provide at least one .torrent file or a magnet link'), 'error')
    return
  }
  isAdding.value = true
  try {
    let added = 0
    // Add each torrent file with its selected files
    for (const t of torrentFileList.value) {
      const allSelected = t.selected.size === t.files.length || t.files.length === 0
      const selectedIndices = allSelected ? null : [...t.selected].map((i) => i + 1)
      const result = await invoke<string>('add_torrent', {
        magnet: null,
        torrentPath: t.path,
        dir: newPath.value || null,
        selectedFiles: selectedIndices,
      })
      try {
        const parsed = JSON.parse(result || '{}')
        if (parsed.gid) added++
      } catch (_) { added++ }
    }
    // Magnet link (if provided) — one additional download
    if (torrentMagnet.value.trim()) {
      const result = await invoke<string>('add_torrent', {
        magnet: torrentMagnet.value.trim(),
        torrentPath: null,
        dir: newPath.value || null,
        selectedFiles: null,
      })
      try {
        const parsed = JSON.parse(result || '{}')
        if (parsed.gid) added++
      } catch (_) { added++ }
    }
    notify(added > 0 ? t('Torrents added!') : t('Torrent added!'), 'success')
    showAddDialog.value = false
    torrentFileList.value = []
    torrentMagnet.value = ''
    newPath.value = ''
    addDialogTab.value = 'url'
    emit('refresh')
  } catch (e: unknown) {
    notify(t('Failed to add torrent') + ': ' + e, 'error')
  } finally {
    isAdding.value = false
  }
}

async function addDownload() {
  if (!newUrl.value.trim()) { notify(t('Please enter a download URL'), 'error'); return }
  if (categoriesEnabled.value && categoryMode.value === 'manual' && !selectedCategory.value.trim()) {
    notify(t('Choose a manual category or turn Categories off.'), 'error')
    return
  }
  if (duplicateDownload.value && allowDuplicate.value) {
    notify(t('Adding as duplicate'), 'info')
  }

  isAdding.value = true
  try {
    const baseDir = newPath.value.trim() || (await loadDefaultDir())
    if (categoriesEnabled.value && !baseDir) {
      notify(t('Could not resolve the download folder.'), 'error')
      return
    }
    const useDuplicate = duplicateDownload.value && allowDuplicate.value
    const resolvedFilename = newFilename.value || detectedFilename.value || newUrl.value
    const finalName = useDuplicate ? withDuplicateSuffix(resolvedFilename, 1) : resolvedFilename
    const categoryDir = resolveCategoryDir({
      enabled: categoriesEnabled.value,
      mode: categoryMode.value,
      selected: selectedCategory.value,
      filename: finalName,
      baseDir,
    })
    const connectionCount = Math.max(1, Number(connections.value) || 1)
    rememberRequestedConnections(newUrl.value.trim(), connectionCount)
    const result = await invoke<string>('add_download', {
      url: newUrl.value,
      filename: useDuplicate ? finalName : newFilename.value || null,
      dir: categoryDir,
      connections: connectionCount,
      schedule: scheduleEnabled.value ? pendingSchedule.value : null,
      proxy: serializeProxy(downloadProxy.value),
    })
    let msg = t('Download added!')
    try {
      const parsed = JSON.parse(result || '{}')
      if (parsed.duplicate) msg = t('Download added (file already existed at destination)')
    } catch (_) {}
    notify(msg, 'success')
    showAddDialog.value = false
    newUrl.value = ''
    newFilename.value = ''
    lastAutoFilledFilename = ''
    newPath.value = ''
    connections.value = 16
    categoriesEnabled.value = false
    categoryMode.value = 'formatted'
    selectedCategory.value = ''
    categoryError.value = ''
    scheduleEnabled.value = false
    pendingSchedule.value = null
    downloadProxy.value = emptyProxyForm()
    addDialogTab.value = 'url'
    allowDuplicate.value = false
    torrentFileList.value = []
    torrentMagnet.value = ''
    showAdvanced.value = false
    wasPastedFromClipboard.value = false
    emit('refresh')
  } catch (e: unknown) {
    notify(t('Failed to add download') + ': ' + e, 'error')
  } finally {
    isAdding.value = false
  }
}

function bringToExistingDownload() {
  if (!duplicateDownload.value) return
  downloadFilter.value = 'all'
  showAddDialog.value = false

  // Move to top via drag reorder state
  const gid = duplicateDownload.value.gid
  const currentGids = sortedDownloads.value.map((d) => d.gid)
  const idx = currentGids.indexOf(gid)
  if (idx > 0) {
    currentGids.splice(idx, 1)
    currentGids.unshift(gid)
    customOrder.value = currentGids
    manualLayout.value = [
      downloadToken(gid),
      ...normalizeManualLayout().filter((token) => token !== downloadToken(gid)),
    ]
  }

  nextTick(() => {
    const el = document.querySelector(`[data-gid="${gid}"]`)
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'center' })
      el.classList.add('highlight-glow')
      setTimeout(() => el.classList.remove('highlight-glow'), 3000)
    }
  })

  notify(`${t('Download already exists')}: ${duplicateDownload.value.filename || gid}`, 'info')
}

// -- Create Group --
const showCreateGroupDialog = ref(false)
const newGroupName = ref('')
const groupNameInput = ref<HTMLInputElement | null>(null)
const renameGroupId = ref<string | null>(null)
const renameGroupName = ref('')
const renameGroupColor = ref('#ef5350')
const newGroupColor = ref('#ef5350')
const newGroupOnComplete = ref('none')
const renameGroupOnComplete = ref('none')
const newGroupSpeedLimit = ref(0)
const renameGroupSpeedLimit = ref(0)
const newGroupDlLimitEnabled = ref(false)
const newGroupDlLimitBytes = ref(0)
const newGroupDlLimitInputValue = ref(1)
const newGroupDlLimitUnit = ref<'KB' | 'MB' | 'GB'>('MB')
const renameGroupDlLimitEnabled = ref(false)
const renameGroupDlLimitBytes = ref(0)
const renameGroupDlLimitUsedBytes = ref(0)
const renameGroupDlLimitInputValue = ref(1)
const renameGroupDlLimitUnit = ref<'KB' | 'MB' | 'GB'>('MB')
const showGroupOptions = ref(false)
const groupColorOptions = [
  '#ef5350', '#e91e63', '#ab47bc', '#7e57c2',
  '#5c6bc0', '#42a5f5', '#26c6da', '#26a69a',
  '#66bb6a', '#9ccc65', '#d4e157', '#ffca28',
  '#ffa726', '#ff7043', '#8d6e63', '#78909c',
]

function formatGroupLimitDisplay(bytes: number): string {
  if (bytes <= 0) return '0'
  if (bytes >= 1073741824) return `${(bytes / 1073741824).toFixed(1)} GB`
  if (bytes >= 1048576) return `${(bytes / 1048576).toFixed(0)} MB`
  return `${(bytes / 1024).toFixed(0)} KB`
}

type GroupDlLimitUnit = 'KB' | 'MB' | 'GB'
const groupDlLimitUnitOptions: { value: GroupDlLimitUnit; label: string }[] = [
  { value: 'KB', label: 'KB' },
  { value: 'MB', label: 'MB' },
  { value: 'GB', label: 'GB' },
]
const groupDlMultipliers: Record<GroupDlLimitUnit, number> = {
  KB: 1024,
  MB: 1048576,
  GB: 1073741824,
}

function groupDlBytesFromInput(value: number, unit: GroupDlLimitUnit): number {
  return Math.round((Number(value) || 0) * groupDlMultipliers[unit])
}

function groupDlUnitForBytes(bytes: number): GroupDlLimitUnit {
  if (bytes >= 1073741824 && bytes % 1073741824 === 0) return 'GB'
  if (bytes >= 1048576 && bytes % 1048576 === 0) return 'MB'
  if (bytes >= 1024 && bytes % 1024 === 0) return 'KB'
  return 'MB'
}

function groupDlValueForBytes(bytes: number, unit: GroupDlLimitUnit): number {
  return bytes > 0 ? bytes / groupDlMultipliers[unit] : 1
}

const onCompleteOptions = computed(() => [
  { value: 'none', label: t('Do nothing') },
  { value: 'close', label: t('Close app') },
  { value: 'sleep', label: t('Sleep') },
  { value: 'shutdown', label: t('Shutdown') },
  { value: 'restart', label: t('Restart') },
])

// -- Group Downloads Sub-modal --
const showGroupDownloads = ref(false)
const editingTarget = ref<'new' | 'edit'>('new')
const newGroupMembers = ref<string[]>([])
const editingGroupMembers = ref<string[]>([])
const groupDownloadUrl = ref('')
const groupDownloadFilename = ref('')
const pendingGroupDownloads = ref<{ url: string; filename: string }[]>([])

function openEditGroup(group: DownloadGroup) {
  renameGroupId.value = group.id
  renameGroupName.value = group.name
  renameGroupColor.value = group.color
  renameGroupOnComplete.value = group.onComplete || 'none'
  renameGroupSpeedLimit.value = group.speedLimit || 0
  renameGroupDlLimitEnabled.value = group.downloadLimitEnabled || false
  renameGroupDlLimitBytes.value = group.downloadLimitBytes || 0
  renameGroupDlLimitUsedBytes.value = group.downloadLimitUsedBytes || 0
  const unit = groupDlUnitForBytes(group.downloadLimitBytes || 0)
  renameGroupDlLimitUnit.value = unit
  renameGroupDlLimitInputValue.value = groupDlValueForBytes(group.downloadLimitBytes || 0, unit)
  editingGroupMembers.value = [...(group.memberGids || [])]
}

function openGroupDownloads(target: 'new' | 'edit') {
  editingTarget.value = target
  groupDownloadUrl.value = ''
  groupDownloadFilename.value = ''
  showGroupDownloads.value = true
}

function getActiveMembers(): string[] {
  return editingTarget.value === 'new' ? newGroupMembers.value : editingGroupMembers.value
}

const displayGroupMembers = computed(() => {
  const members = getActiveMembers()
  return members.map((m: string) => {
    if (m.startsWith('pending-')) {
      const pending = pendingGroupDownloads.value.find((_, i) => `pending-${i}` === m)
      return { gid: m, filename: pending?.url || t('Pending...'), status: 'pending', isPending: true }
    }
    const d = downloads.value.find((dl) => dl.gid === m)
    return { gid: m, filename: d?.filename || m, status: d?.status || 'unknown', isPending: false }
  })
})

function removeGroupMember(gid: string) {
  const members = getActiveMembers()
  const idx = members.indexOf(gid)
  if (idx !== -1) members.splice(idx, 1)
  if (gid.startsWith('pending-')) {
    const pendingIdx = parseInt(gid.split('-')[1])
    if (!isNaN(pendingIdx) && pendingGroupDownloads.value[pendingIdx]) {
      pendingGroupDownloads.value.splice(pendingIdx, 1)
    }
  } else if (editingTarget.value === 'edit' && renameGroupId.value) {
    void removeMember(renameGroupId.value, gid)
  }
}

function addPendingGroupDownload() {
  if (!groupDownloadUrl.value.trim()) return
  const idx = pendingGroupDownloads.value.length
  pendingGroupDownloads.value.push({ url: groupDownloadUrl.value.trim(), filename: groupDownloadFilename.value.trim() })
  getActiveMembers().push(`pending-${idx}`)
  groupDownloadUrl.value = ''
  groupDownloadFilename.value = ''
}

async function submitPendingGroupDownloads(): Promise<string[]> {
  const gids: string[] = []
  for (const pending of pendingGroupDownloads.value) {
    try {
      const result = await invoke<string>('add_download', {
        url: pending.url,
        filename: pending.filename || null,
        dir: null,
        connections: 1,
        schedule: null,
      })
      try {
        const parsed = JSON.parse(result || '{}')
        if (parsed.gid) gids.push(parsed.gid)
      } catch (_) {}
    } catch (e) {
      notify(t('Failed to add download') + ': ' + e, 'error')
    }
  }
  pendingGroupDownloads.value = []
  return gids
}

function addGroupDownload() {
  if (!groupDownloadUrl.value.trim()) return
  addPendingGroupDownload()
}

async function confirmCreateGroup() {
  if (!newGroupName.value.trim()) return
  const gids = await submitPendingGroupDownloads()
  const realMembers = newGroupMembers.value.filter(m => !m.startsWith('pending-')).concat(gids)
  const speedLimit = newGroupSpeedLimit.value
  const g = createGroup(
    newGroupName.value,
    newGroupColor.value,
    realMembers,
    newGroupOnComplete.value,
    speedLimit,
    newGroupDlLimitEnabled.value,
    newGroupDlLimitBytes.value,
  )
  if (speedLimit > 0) {
    await applyGroupLimits(g)
  }
  newGroupName.value = ''
  newGroupOnComplete.value = 'none'
  newGroupSpeedLimit.value = 0
  newGroupDlLimitEnabled.value = false
  newGroupDlLimitBytes.value = 0
  newGroupDlLimitInputValue.value = 1
  newGroupDlLimitUnit.value = 'MB'
  newGroupMembers.value = []
  showCreateGroupDialog.value = false
  notify(t('Group created!'), 'success')
  remountSortablesIfGrabbed()
  if (gids.length > 0) emit('refresh')
}

async function confirmRenameGroup() {
  if (!renameGroupId.value || !renameGroupName.value.trim()) return
  const gids = await submitPendingGroupDownloads()
  const g = groups.value.find((g) => g.id === renameGroupId.value)
  if (g) {
    const realMembers = editingGroupMembers.value.filter(m => !m.startsWith('pending-')).concat(gids)
    const oldSpeedLimit = g.speedLimit || 0
    const newSpeedLimit = renameGroupSpeedLimit.value

    updateGroup(g.id, {
      name: renameGroupName.value.trim(),
      color: renameGroupColor.value,
      onComplete: renameGroupOnComplete.value,
      memberGids: realMembers,
      speedLimit: newSpeedLimit,
      downloadLimitEnabled: renameGroupDlLimitEnabled.value,
      downloadLimitBytes: renameGroupDlLimitBytes.value,
      downloadLimitUsedBytes: g.downloadLimitUsedBytes,
    })

    if (newSpeedLimit !== oldSpeedLimit) {
      await applyGroupLimits({ ...g, memberGids: realMembers, speedLimit: newSpeedLimit })
    }
    // Reset used bytes if limit was disabled
    if (!renameGroupDlLimitEnabled.value && g.downloadLimitEnabled) {
      resetGroupDownloadLimit(g.id)
    }
  }
  renameGroupId.value = null
  notify(t('Group updated'), 'success')
  remountSortablesIfGrabbed()
  if (gids.length > 0) emit('refresh')
}

// -- Group On-Complete Actions --
const triggeredGroups = new Set<string>()

async function executeGroupAction(action: string) {
  try {
    if (action === 'close') {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      getCurrentWindow().close()
    } else if (action === 'sleep') {
      const { Command } = await import('@tauri-apps/plugin-shell')
      await Command.create('exec', ['powershell', '-Command', 'rundll32.exe powrprof.dll,SetSuspendState 0,1,0']).execute()
    } else if (action === 'shutdown') {
      const { Command } = await import('@tauri-apps/plugin-shell')
      await Command.create('exec', ['powershell', '-Command', 'shutdown /s /t 30']).execute()
    } else if (action === 'restart') {
      const { Command } = await import('@tauri-apps/plugin-shell')
      await Command.create('exec', ['powershell', '-Command', 'shutdown /r /t 30']).execute()
    }
  } catch (e) {
    notify(t('Failed to execute action') + ': ' + e, 'error')
  }
}

watch(downloads, () => {
  const eligibleGroups = groups.value.filter(
    (group) =>
      group.onComplete &&
      group.onComplete !== 'none' &&
      group.memberGids &&
      group.memberGids.length > 0,
  )
  if (eligibleGroups.length === 0) return

  const downloadsByGid = new Map<string, Download>()
  for (const d of downloads.value) downloadsByGid.set(d.gid, d)
  for (const group of eligibleGroups) {
    if (triggeredGroups.has(group.id)) continue
    const allComplete = group.memberGids.every((gid: string) => {
      const d = downloadsByGid.get(gid)
      return d && d.status === 'complete'
    })
    if (allComplete) {
      triggeredGroups.add(group.id)
      notify(`${t('Group complete')}: "${group.name}" ${t('Executing')} ${group.onComplete}`, 'info')
      executeGroupAction(group.onComplete)
    }
  }
})

async function deleteGroup(id: string) {
  const g = removeGroup(id)
  if (g?.memberGids?.length) {
    for (const gid of g.memberGids) {
      await applyMemberLimit(gid, 0)
    }
  }
  notify(t('Group deleted'), 'info')
  remountSortablesIfGrabbed()
}

function toggleGroupCollapse(id: string) {
  const g = groups.value.find((g) => g.id === id)
  if (g) {
    g.collapsed = !g.collapsed
    saveGroups()
  }
  remountSortablesIfGrabbed()
}

async function addToGroup(groupId: string, gid: string) {
  const dl = downloads.value.find((d) => d.gid === gid)
  if (dl?.status === 'complete') {
    notify(t('Cannot add completed download to group'), 'error')
    return
  }
  const g = await addMember(groupId, gid)
  if (g) {
    notify(`${t('Added to')}: "${g.name}"`, 'success')
    window.dispatchEvent(new Event('moonhunt:groups-changed'))
  }
}

async function removeMemberFromGroup(groupId: string, gid: string) {
  const g = await removeMember(groupId, gid)
  if (g) notify(`${t('Removed from')}: "${g.name}"`, 'success')
}



// -- Scroll to top --
const showScrollTop = ref(false)

function scrollToTop() {
  const container = document.querySelector<HTMLElement>('.content-area') || document.documentElement
  container.scrollTo({ top: 0, behavior: 'smooth' })
}

// -- Details Modal --
const showDetailsModal = ref(false)
const selectedDownload = ref<Download | null>(null)
const selectedDownloadDetails = ref<DownloadDetails | null>(null)
const activeDetailsTab = ref('general')

function showDownloadDetails(download: Download) {
  selectedDownload.value = download
  selectedDownloadDetails.value = null
  activeDetailsTab.value = 'general'
  detailProxy.value = parseProxyString(download.proxy)
  proxySaveMessage.value = ''
  proxySaveOk.value = false
  showDetailsModal.value = true
  // Fetch detailed info
  invoke<any>('get_download_details', { gid: download.gid }).then(details => {
    selectedDownloadDetails.value = details
  }).catch((e: unknown) => {
    console.error('Failed to fetch details:', e)
  })
}

// Per-download proxy editor in the details modal Network tab.
const detailProxy = ref<ProxyFormValue>(emptyProxyForm())
const proxyApplying = ref(false)
const proxySaveMessage = ref('')
const proxySaveOk = ref(false)

async function applyDetailProxy() {
  const gid = selectedDownload.value?.gid
  if (!gid || proxyApplying.value) return
  proxyApplying.value = true
  proxySaveMessage.value = ''
  try {
    await invoke('set_download_proxy', {
      gid,
      proxy: {
        enabled: detailProxy.value.enabled,
        proxy_type: detailProxy.value.type,
        proxy_host: detailProxy.value.host,
        proxy_port: detailProxy.value.port,
        proxy_user: detailProxy.value.user,
        proxy_pass: detailProxy.value.pass,
      },
    })
    proxySaveOk.value = true
    proxySaveMessage.value = t('Settings saved successfully!')
    if (selectedDownload.value) {
      selectedDownload.value.proxy = serializeProxy(detailProxy.value) || ''
    }
    emit('refresh')
  } catch (e) {
    proxySaveOk.value = false
    proxySaveMessage.value = `${t('Failed to save settings')}: ${e}`
  } finally {
    proxyApplying.value = false
  }
}

// -- Error Details Dialog --
const showErrorDialog = ref(false)
const selectedError = ref<{ filename: string; message: string }>({ filename: '', message: '' })

function showErrorDetails(download: Download) {
  selectedError.value = { filename: download.filename, message: download.errorMessage || `Error code: ${download.errorCode}` }
  showErrorDialog.value = true
}

// -- Download actions --
async function pauseDownload(gid: string) {
  try {
    if (gid.startsWith('yt-dlp-direct-')) {
      // Direct media downloads can't be paused mid-file; cancel the process and
      // remove the row. The file (if partially downloaded) stays on disk.
      await invoke('cancel_yt_download', { gid })
      notify(t('Media download paused (cancelled)'), 'info')
    } else {
      await invoke('pause_download', { gid })
    }
    emit('refresh')
  } catch (e) { notify(t('Failed to pause') + ': ' + e, 'error') }
}

async function resumeDownload(gid: string) {
  try {
    if (gid.startsWith('yt-dlp-direct-')) {
      // Paused media downloads are re-launchable; re-run from the DB record.
      const res = await invoke<string>('resume_yt_download', { gid })
      notify(res, 'success')
    } else {
      await invoke('resume_download', { gid })
    }
    emit('refresh')
  } catch (e) { notify(t('Failed to resume') + ': ' + e, 'error') }
}

async function retryDownload(download: Download) {
  try {
    if (download.gid.startsWith('yt-dlp-direct-')) {
      // Media downloads can't be resumed from aria2; cancel the old row and
      // re-run the source URL directly.
      await invoke('cancel_yt_download', { gid: download.gid })
      if (download.url && /^https?:\/\//i.test(download.url)) {
        await invoke('add_yt_download', {
          url: download.url, format: 'mp4', quality: 'best',
          path: download.path ? download.path.split(/[\\/]/).slice(0, -1).join('/') : undefined,
          browser: undefined, format_id: undefined,
        })
      }
    } else {
      await invoke('retry_download', { gid: download.gid })
    }
    emit('refresh')
  } catch (e) { notify(t('Failed to retry') + ': ' + e, 'error') }
}

async function redownloadFile(download: Download) {
  try {
    rememberRequestedConnections(download.url, download.connections ?? 1)
    // Remove old download from DB to prevent it showing in history with error message
    await invoke('remove_download_from_db', { gid: download.gid })
    // Also try to remove from aria2 if it's still there
    await invoke('remove_download', { gid: download.gid }).catch(() => {})
    // Add new download with same settings
    await invoke('add_download', {
      url: download.url,
      filename: download.filename,
      connections: download.connections ?? 1,
      dir: download.path ? download.path.split(/[\\/]/).slice(0, -1).join('/') : undefined,
      proxy: download.proxy || null,
    })
    emit('refresh')
  } catch (e) { notify(t('Failed to redownload') + ': ' + e, 'error') }
}

function showInFolder(download: Download) {
  invoke('show_download_in_folder', { gid: download.gid, filePath: download.path || null, filename: download.filename || null }).catch((e: unknown) => {
    notify(t('Failed to open folder') + ': ' + e, 'error')
  })
}

function openDownloadFile(download: Download) {
  invoke('open_download_file', { gid: download.gid, filePath: download.path || null, filename: download.filename || null }).catch((e: unknown) => {
    notify(t('Failed to open file') + ': ' + e, 'error')
  })
}

async function cancelDownload(gid: string) {
  try {
    emit('cancel', gid)
  } catch (e) { notify(t('Failed to cancel') + ': ' + e, 'error') }
}

async function removeDownload(gid: string) {
  try {
    emit('remove', gid)
  } catch (e) { notify(t('Failed to remove') + ': ' + e, 'error') }
}

// -- Helper functions --
function getRequestedConnections(download: Download) {
  return download.connections ?? 1
}

function withDuplicateSuffix(name: string, count = 1): string {
  const dot = name.lastIndexOf('.')
  if (dot <= 0) return `${name} (${count})`
  return `${name.slice(0, dot)} (${count})${name.slice(dot)}`
}

function capitalizeStatus(status?: string | null) {
  const s = status || ''
  if (!s) return ''
  const labels: Record<string, string> = {
    active: t('Active'),
    complete: t('Completed'),
    paused: t('Paused'),
    error: t('Failed'),
    waiting: t('Waiting'),
    cancelled: t('Cancelled'),
    stopped: t('Stopped'),
  }
  return labels[s] || s.charAt(0).toUpperCase() + s.slice(1)
}

function getStatusTextStyle(status?: string | null) {
  const colors: Record<string, string> = {
    active: 'color: var(--status-active)', complete: 'color: var(--status-complete)', paused: 'color: var(--status-paused)', error: 'color: var(--status-error)', waiting: 'color: var(--status-waiting)', cancelled: 'color: var(--status-cancelled)'
  }
  return colors[status || ''] || ''
}

function getStatusIcon(status?: string | null) {
  const icons: Record<string, string> = {
    active: 'play_circle', complete: 'check_circle', paused: 'pause_circle', error: 'error', waiting: 'schedule', stopped: 'stop_circle', cancelled: 'cancel'
  }
  return icons[status || ''] || 'fiber_manual_record'
}

function getFileIcon(filename: string | undefined): string {
  if (!filename) return 'insert_drive_file'
  const ext = filename.split('.').pop()?.toLowerCase()
  if (!ext) return 'insert_drive_file'
  const media = ['mp4', 'mkv', 'webm', 'mov', 'avi', 'mp3', 'flac', 'wav', 'm4a', 'ogg']
  const archives = ['zip', 'rar', '7z', 'tar', 'gz', 'bz2', 'xz']
  const images = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'svg', 'bmp', 'ico']
  const docs = ['pdf', 'doc', 'docx', 'txt', 'md', 'rtf', 'odt']
  const code = ['js', 'ts', 'py', 'rs', 'go', 'cpp', 'c', 'h', 'java', 'cs', 'php', 'rb']
  if (media.includes(ext)) return 'movie'
  if (archives.includes(ext)) return 'archive'
  if (images.includes(ext)) return 'image'
  if (docs.includes(ext)) return 'description'
  if (code.includes(ext)) return 'code'
  return 'insert_drive_file'
}

// -- Bulk Actions --
function pauseAll() {
  downloads.value.forEach(d => {
    if (d.status === 'active' || d.status === 'waiting') {
      pauseDownload(d.gid)
    }
  })
}

function resumeAll() {
  downloads.value.forEach(d => {
    if (d.status === 'paused' || d.status === 'waiting') {
      resumeDownload(d.gid)
    }
  })
}

async function clearHistory() {
  try {
    const completed = downloads.value.filter((d) => d.status === 'complete' || d.status === 'error')
    for (const d of completed) {
      await invoke('remove_download', { gid: d.gid }).catch(() => {})
      await invoke('remove_download_from_db', { gid: d.gid }).catch(() => {})
    }
    emit('refresh')
    notify(t('History cleared'), 'success')
  } catch (e: unknown) {
    notify(t('Failed to clear history') + ': ' + e, 'error')
  }
}

function getDownloadFolder(): string {
  if (selectedDownload?.value?.path) {
    return selectedDownload.value.path.split(/[\\/]/).slice(0, -1).join('\\') || 'â€”'
  }
  if (selectedDownloadDetails?.value?.files?.[0]?.path) {
    return selectedDownloadDetails.value.files[0].path.split(/[\\/]/).slice(0, -1).join('\\') || 'â€”'
  }
  return 'â€”'
}

function copyToClipboard(text?: string | null): void {
  if (!text || text === 'â€”') return
  navigator.clipboard.writeText(text).then(() => {
    notify(t('Copied to clipboard'), 'success')
  }).catch(() => {
    notify(t('Failed to copy'), 'error')
  })
}

</script>

<style scoped>
/* â”€â”€ Page â”€â”€ */
.downloads-page { color: var(--text); }

/* â”€â”€ Hero (copied from SettingsPage) â”€â”€ */
.settings-hero {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 24px;
  padding: 16px 28px;
  margin-bottom: 18px;
  border-radius: 20px;
  overflow: visible;
  position: relative;
  background: linear-gradient(135deg, rgba(var(--accent-rgb), 0.12), var(--bg-card));
  border: 1px solid var(--border-strong);
  box-shadow: var(--shadow-card);
  color: var(--text);
}
.light .settings-hero {
  background: linear-gradient(135deg, rgba(var(--accent-rgb), 0.25), var(--bg-card));
}

.hero-left {
  display: flex;
  align-items: center;
  gap: 20px;
  min-width: 0;
}

.hero-icon-area {
  position: relative;
  width: 56px;
  height: 56px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.hero-orb {
  position: absolute;
  inset: 0;
  border-radius: 16px;
  background: linear-gradient(135deg, rgba(var(--accent-rgb), 0.25), rgba(var(--accent-rgb), 0.08));
  border: 1px solid rgba(var(--accent-rgb), 0.18);
  animation: orbPulse 4s ease-in-out infinite;
}

@keyframes orbPulse {
  0%, 100% { box-shadow: 0 0 12px rgba(var(--accent-rgb), 0.15); }
  50% { box-shadow: 0 0 24px rgba(var(--accent-rgb), 0.3); }
}

.hero-main-icon {
  position: relative;
  z-index: 1;
  font-size: 28px;
  color: var(--accent-strong);
}

.hero-copy {
  min-width: 0;
}

.hero-copy h1 {
  font-size: clamp(1.5rem, 3vw, 2rem);
  font-weight: 800;
  margin: 0;
  letter-spacing: -0.03em;
  line-height: 1.1;
}

.hero-copy .filter-bar {
  margin-top: 10px;
  width: fit-content;
}

.hero-sub {
  color: var(--text-muted);
  font-size: 13.5px;
  line-height: 1.55;
  margin: 6px 0 0;
  max-width: 380px;
}

/* â”€â”€ Hero Bottom Bar â”€â”€ */
.hero-bottom-bar {
  position: absolute;
  bottom: 16px;
  right: 16px;
  pointer-events: none;
}

.bar-box {
  pointer-events: auto;
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 6px;
  background: var(--toolbar-bg);
  border: 1px solid var(--toolbar-border);
  border-radius: 10px;
}

.btn-add-icon {
  pointer-events: auto;
  width: 36px;
  height: 36px;
  border-radius: 50%;
  border: none;
  background: linear-gradient(135deg, var(--accent), rgba(var(--accent-rgb), 0.7));
  color: #0a0e1a;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.2s ease;
  box-shadow: 0 2px 8px rgba(var(--accent-rgb), 0.3);
}

.btn-add-icon:hover {
  transform: scale(1.05);
  box-shadow: 0 4px 16px rgba(var(--accent-rgb), 0.45);
}

.btn-add-icon:active {
  transform: scale(0.98);
}

.btn-add-icon .material-icons {
  font-size: 20px;
}

.btn-action-icon {
  pointer-events: auto;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s ease;
}

.btn-action-icon.active {
  background: var(--toolbar-active);
  color: var(--accent-strong);
}

.bar-divider {
  width: 1px;
  height: 20px;
  background: rgba(255, 255, 255, 0.2);
  margin: 0 8px;
}

.btn-action-icon:hover:not(.active) {
  color: var(--accent-strong);
  background: transparent;
}

.btn-action-icon:active {
  transform: scale(0.98);
}

.btn-action-icon .material-icons {
  font-size: 18px;
}

/* â”€â”€ Toolbar â”€â”€ */
.filter-bar {
  position: relative;
  display: flex;
  align-items: center;
  gap: 2px;
  background: var(--toolbar-bg);
  padding: 10px 6px;
  border-radius: 10px;
  border: 1px solid var(--toolbar-border);
}
.filter-btn-indicator {
  position: absolute;
  inset-inline-start: 6px;
  top: 10px;
  width: 28px;
  height: 28px;
  background: var(--toolbar-active);
  border-radius: 6px;
  box-shadow: var(--toolbar-active-shadow);
  pointer-events: none;
  transition: transform 0.22s cubic-bezier(0.4, 0, 0.2, 1);
  z-index: 0;
}
.filter-bar.pos-1 .filter-btn-indicator { transform: translateX(calc(100% + 2px)); }
.filter-bar.pos-2 .filter-btn-indicator { transform: translateX(calc(200% + 4px)); }
html[dir='rtl'] .filter-bar.pos-1 .filter-btn-indicator { transform: translateX(calc(-100% - 2px)); }
html[dir='rtl'] .filter-bar.pos-2 .filter-btn-indicator { transform: translateX(calc(-200% - 4px)); }
.filter-btn {
  background: none;
  border: none;
  color: var(--text-muted);
  width: 28px;
  height: 28px;
  padding: 0;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
  position: relative;
  z-index: 1;
}
.filter-btn.active { background: transparent; color: var(--accent-strong); box-shadow: none; }
.filter-btn:hover:not(.active) { color: var(--accent-strong); background: transparent; }
.filter-btn .material-icons {
  font-size: 18px;
  line-height: 1;
  display: block;
  pointer-events: none;
  overflow: visible;
}
.btn-tooltip {
  position: absolute; bottom: calc(100% + 6px); left: 50%; transform: translateX(-50%);
  background: var(--bg-card); color: var(--text); font-size: 11px; font-weight: 600;
  padding: 4px 10px; border-radius: 6px; border: 1px solid var(--border-strong);
  box-shadow: var(--shadow-md); white-space: nowrap; opacity: 0; pointer-events: none;
  transition: opacity 0.15s; z-index: 100;
}
.filter-btn:hover .btn-tooltip { opacity: 1; }

.hero-actions { display: flex; align-items: center; gap: 6px; }
.hero-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  border-radius: 10px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
  border: none;
}
.hero-btn i { font-size: 18px; }
.hero-btn.primary { background: var(--accent); color: #0a0e1a; }
.hero-btn.primary:hover { opacity: 0.9; transform: translateY(-1px); }
.hero-btn.outlined { background: var(--toolbar-bg); color: var(--text); border: 1px solid var(--toolbar-border); }
.hero-btn.outlined:hover { background: rgba(var(--accent-rgb), 0.1); color: var(--accent-strong); }

.view-toggle { display: flex; gap: 2px; background: var(--toolbar-bg); padding: 3px; border-radius: 10px; flex-shrink: 0; border: 1px solid var(--toolbar-border); }
.view-btn { background: none; border: none; color: var(--text-dim); width: 30px; height: 30px; border-radius: 7px; cursor: pointer; display: flex; align-items: center; justify-content: center; transition: all 0.15s; }
.view-btn i { font-size: 18px; }
.view-btn.active { background: var(--toolbar-active); color: var(--accent-strong); box-shadow: var(--toolbar-active-shadow); }
.view-btn:hover:not(.active) { color: var(--text); }

.sort-wrapper { position: relative; flex-shrink: 0; }
.sort-btn {
  display: flex;
  align-items: center;
  gap: 5px;
  background: var(--toolbar-bg);
  border: 1px solid var(--toolbar-border);
  border-radius: 8px;
  color: var(--text-dim);
  padding: 6px 12px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
}
.sort-btn i { font-size: 16px; }
.sort-btn:hover { background: var(--toolbar-active); color: var(--text); }
.sort-menu {
  position: absolute; top: 100%; right: 0; margin-top: 6px;
  background: var(--bg-card); border: 1px solid var(--border-strong);
  border-radius: 10px; box-shadow: var(--shadow-lg);
  padding: 6px; min-width: 160px; z-index: 50;
}
.sort-menu-label { font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-dim); padding: 6px 10px 4px; opacity: 0.6; }
.sort-menu-item {
  display: block; width: 100%; background: none; border: none;
  color: var(--text); padding: 7px 10px; font-size: 13px;
  border-radius: 6px; cursor: pointer; text-align: left; transition: background 0.1s;
}
.sort-menu-item:hover { background: rgba(var(--accent-rgb), 0.06); }
.sort-menu-item.active { font-weight: 600; color: var(--accent-strong); }
.sort-menu-divider { height: 1px; background: var(--border-strong); margin: 4px 8px; }
.sort-option {
  display: flex; align-items: center; gap: 8px;
  width: 100%; background: none; border: none;
  color: var(--text); padding: 7px 10px; font-size: 13px;
  border-radius: 6px; cursor: pointer; text-align: left; transition: background 0.1s;
}
.sort-option:hover { background: rgba(var(--accent-rgb), 0.06); }
.sort-option.active { font-weight: 600; }
.sort-check { font-size: 16px; color: var(--accent-strong); }

/* â”€â”€ Color picker â”€â”€ */
.color-picker-row { display: flex; gap: 6px; flex-wrap: wrap; }
.color-swatch {
  width: 24px; height: 24px; border-radius: 50%; border: 2px solid transparent;
  cursor: pointer; transition: all 0.15s ease; padding: 0;
}
.color-swatch:hover { transform: scale(1.15); }
.color-swatch.selected { border-color: #fff; box-shadow: 0 0 0 2px var(--accent); transform: scale(1.1); }

/* â”€â”€ Sticky toolbar â”€â”€ */
.sticky-toolbar {
  position: fixed; top: 50px; left: var(--sidebar-width, 72px); right: 0; z-index: 900;
  display: flex; align-items: center; gap: 10px;
  padding: 6px 14px;
  background: var(--toolbar-bg); backdrop-filter: blur(12px);
  border-bottom: 1px solid var(--toolbar-border);
}
html[dir='rtl'] .sticky-toolbar {
  left: 0;
  right: var(--sidebar-width, 72px);
}
.sticky-fade-enter-active, .sticky-fade-leave-active { transition: opacity 0.2s ease; }
.sticky-fade-enter-from, .sticky-fade-leave-to { opacity: 0; }

/* â”€â”€ Views â”€â”€ */
.downloads-list, .downloads-compact { display: flex; flex-direction: column; gap: 6px; padding-bottom: 40px; }
.top-level-drop-zone {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  min-height: 60px;
  border: 2px dashed transparent;
  border-radius: 12px;
  color: transparent;
  font-size: 13px;
  font-weight: 500;
  margin: 8px 0;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.15s ease, border-color 0.15s ease, color 0.15s ease;
}
.top-level-drop-zone.is-active {
  border-color: var(--border);
  color: var(--text-dim);
  opacity: 1;
  pointer-events: auto;
}
.top-level-drop-zone .material-icons { font-size: 18px; }


/* â”€â”€ Groups â”€â”€ */
.group-options-toggle.active { color: var(--accent-strong); background: rgba(var(--accent-rgb), 0.1); }
.group-options-toggle .material-icons { transition: transform 0.2s ease; font-size: 18px; }
.group-options-toggle .material-icons.rotated { transform: rotate(-90deg); }
.group-nd-modal .nd-field :deep(.custom-dropdown .custom-dropdown-trigger) {
  height: 36px;
  padding: 0 10px;
  background: rgba(255, 255, 255, 0.025);
  border: 1px solid var(--border);
  border-radius: 8px;
  font-size: 12px;
}
.group-nd-modal .nd-field :deep(.custom-dropdown .custom-dropdown-trigger:hover) {
  background: rgba(255, 255, 255, 0.035);
  border-color: rgba(255, 255, 255, 0.1);
}
.group-downloads-btn {
  display: flex; align-items: center; justify-content: flex-start; gap: 8px; width: 100%;
  padding: 8px 10px; color: var(--text); font-size: 12px; cursor: pointer;
}
.group-downloads-btn .material-icons { font-size: 17px; color: var(--accent-strong); }
.group-member-count {
  margin-left: auto; background: rgba(var(--accent-rgb), 0.12); color: var(--accent-strong);
  font-size: 11px; font-weight: 600; padding: 2px 8px; border-radius: 999px;
}
.nd-toggle-row { display: flex; align-items: center; gap: 10px; }
.nd-toggle-label { font-size: 12px; font-weight: 600; color: var(--text); }
.nd-toggle-hint { margin-left: auto; font-size: 11px; color: var(--text-dim); font-weight: 500; }
.group-downloads-modal { max-width: 400px; }
.nd-member-list { display: flex; flex-direction: column; gap: 2px; max-height: 240px; overflow-y: auto; margin: 0 -4px; padding: 0 4px; }
.nd-member-empty { text-align: center; padding: 24px 0; color: var(--text-dim); font-size: 12px; }
.group-member-row {
  display: flex; align-items: center; gap: 10px; padding: 8px 10px;
  border-radius: 8px; transition: background 0.1s;
}
.group-member-row:hover { background: rgba(255,255,255,0.04); }
.group-member-info { display: flex; flex-direction: column; gap: 2px; min-width: 0; flex: 1; }
.group-member-name { font-size: 12px; font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.group-member-status { font-size: 11px; }
.group-member-remove {
  background: none; border: none; color: var(--text-dim); cursor: pointer;
  padding: 4px; border-radius: 4px; display: flex; transition: all 0.15s ease;
}
.group-member-remove:hover { color: var(--status-error); background: rgba(239, 83, 80, 0.1); }
.group-member-status.pending { color: var(--text-dim); font-style: italic; }
.group-container {
  position: relative;
  border-radius: 12px;
  overflow: hidden;
  background: color-mix(in srgb, var(--group-color, var(--accent)) 5%, transparent);
  border: 1px solid color-mix(in srgb, var(--group-color, var(--accent)) 22%, transparent);
  box-shadow: var(--shadow-sm);
  transition: border-color 0.2s ease, box-shadow 0.2s ease, transform 0.2s ease;
}
.group-container:hover {
  border-color: color-mix(in srgb, var(--group-color, var(--accent)) 34%, transparent);
  background: color-mix(in srgb, var(--group-color, var(--accent)) 7%, transparent);
  box-shadow: var(--shadow-md);
}
.group-drop-overlay {
  position: absolute;
  inset: 0;
  z-index: 20;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 6px;
  background: color-mix(in srgb, var(--group-color, var(--accent)) 14%, rgba(6, 8, 14, 0.78));
  color: #fff;
  font-size: 13px;
  font-weight: 700;
  letter-spacing: 0.02em;
  white-space: nowrap;
  text-align: center;
  text-shadow: 0 1px 4px rgba(0, 0, 0, 0.35);
  backdrop-filter: blur(2px);
  pointer-events: none;
}
.group-drop-overlay .material-icons { font-size: 22px; }
.group-header {
  position: relative;
  display: flex;
  flex-direction: column;
  justify-content: flex-start;
  align-items: stretch;
  gap: 4px;
  padding: 12px 16px;
  min-height: 52px;
  cursor: pointer;
  user-select: none;
  background: transparent;
  border-bottom: 1px solid color-mix(in srgb, var(--group-color, var(--accent)) 14%, transparent);
  transition: min-height 0.28s cubic-bezier(0.4, 0, 0.2, 1), background 0.2s ease, border-color 0.2s ease, padding 0.25s ease;
}
.group-container.group-collapsed .group-header { min-height: 116px; }
.group-container.group-collapsed .group-meta { flex-wrap: nowrap; overflow: hidden; }
.group-container:not(.group-collapsed) .group-header { padding: 8px 14px; min-height: 46px; }
.group-container:not(.group-collapsed) .group-avatar { width: 28px; height: 28px; border-radius: 8px; }
.group-container:not(.group-collapsed) .group-avatar .material-icons { font-size: 16px; }
.group-container:not(.group-collapsed) .group-name { font-size: 12px; }
.group-container:not(.group-collapsed) .group-meta { flex-wrap: nowrap; overflow: hidden; }
.group-header-top { display: flex; align-items: center; justify-content: space-between; gap: 10px; min-width: 0; flex-shrink: 0; }
.group-header:hover { background: color-mix(in srgb, var(--group-color, var(--accent)) 4%, transparent); }
.group-header.drag-over {
  box-shadow: inset 0 0 0 2px color-mix(in srgb, var(--group-color, var(--accent)) 55%, transparent);
  filter: brightness(1.15);
}
.group-header-left { display: flex; align-items: center; gap: 10px; min-width: 0; flex: 1; }
.group-avatar {
  width: 36px; height: 36px; border-radius: 10px; flex-shrink: 0;
  display: flex; align-items: center; justify-content: center;
  color: var(--group-color, var(--accent));
  background: color-mix(in srgb, var(--group-color, var(--accent)) 12%, transparent);
  border: 1px solid color-mix(in srgb, var(--group-color, var(--accent)) 26%, transparent);
}
.group-avatar .material-icons { font-size: 19px; }
.group-heading { min-width: 0; flex: 1; }
.group-title-row { display: flex; align-items: center; gap: 6px; min-width: 0; }
.group-toggle-icon { font-size: 17px; color: var(--group-color, var(--accent)); flex-shrink: 0; transition: transform 0.2s ease, color 0.2s ease; }
.group-toggle-icon.rotated { transform: rotate(-90deg); }
.group-name { font-size: 13.5px; font-weight: 600; letter-spacing: 0.01em; color: var(--text); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.group-meta { display: flex; align-items: center; flex-wrap: wrap; gap: 5px; margin-top: 2px; }
.group-count, .group-size, .group-added, .group-limit-badge {
  display: inline-flex; align-items: center; gap: 4px;
  font-size: 10.5px; font-weight: 600; line-height: 1;
  padding: 2px 8px; border-radius: 999px;
  white-space: nowrap;
}
.group-count { color: var(--group-color, var(--accent)); background: color-mix(in srgb, var(--group-color, var(--accent)) 12%, transparent); }
.group-size { color: var(--text-muted); background: color-mix(in srgb, var(--group-color, var(--accent)) 7%, transparent); }
.group-added { color: var(--text-dim); background: color-mix(in srgb, var(--group-color, var(--accent)) 7%, transparent); }
.group-limit-badge { color: var(--text-secondary); background: color-mix(in srgb, var(--group-color, var(--accent)) 7%, transparent); }
.group-count .material-icons, .group-size .material-icons, .group-added .material-icons, .group-limit-badge .material-icons { font-size: 12px; }
.group-preview {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  justify-content: flex-start;
  gap: 2px;
  overflow: hidden;
  padding-top: 0;
}
.group-preview-row { display: flex; flex-direction: column; gap: 2px; min-width: 0; flex-shrink: 0; }
.group-preview-row-top { display: flex; align-items: center; gap: 8px; min-width: 0; }
.group-preview-name {
  flex: 1; min-width: 0;
  font-size: 10px; font-weight: 600; line-height: 1.1;
  color: var(--text-muted);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.group-preview-size {
  flex-shrink: 0;
  font-size: 8.5px; font-weight: 700;
  color: var(--text-dim);
  white-space: nowrap;
}
.group-preview-track {
  height: 2px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--group-color, var(--accent)) 10%, transparent);
  overflow: hidden;
}
.group-preview-fill {
  height: 100%;
  border-radius: 999px;
  background: var(--group-color, var(--accent));
  transition: width 0.3s ease;
}
.group-preview-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  min-height: 0;
  color: var(--text-dim);
  font-size: 10.5px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
}
.group-preview-empty .material-icons { font-size: 14px; }
.group-preview-more { flex-shrink: 0; font-size: 9px; font-weight: 700; color: var(--group-color, var(--accent)); }
.group-preview-fade-enter-active,
.group-preview-fade-leave-active { transition: opacity 0.18s ease, transform 0.18s ease; }
.group-preview-fade-enter-from,
.group-preview-fade-leave-to { opacity: 0; transform: translateY(6px); }
.group-body-fade-enter-active,
.group-body-fade-leave-active { transition: opacity 0.16s ease, transform 0.16s ease; }
.group-body-fade-enter-from,
.group-body-fade-leave-to { opacity: 0; transform: translateY(-4px); }
.group-progress { position: absolute; left: 16px; right: 16px; bottom: 0; height: 4px; background: var(--hover-bg); border-radius: 2px 2px 0 0; overflow: hidden; }
.group-progress-fill { height: 100%; background: var(--group-color, var(--accent)); border-radius: 2px 2px 0 0; transition: width 0.3s ease; }
.limit-usage-bar { display: flex; flex-direction: column; gap: 6px; }
.limit-usage-track { height: 6px; background: rgba(255, 255, 255, 0.06); border-radius: 3px; overflow: hidden; }
.limit-usage-fill { height: 100%; background: var(--accent); border-radius: 3px; transition: width 0.4s ease; }
.limit-usage-text { font-size: 12px; color: var(--text-dim); font-weight: 600; }
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

.field .nd-label {
  display: flex;
  align-items: center;
  gap: 5px;
  text-transform: none;
  letter-spacing: 0.01em;
}

.field-hint {
  display: block;
  margin-top: 6px;
  color: var(--text-dim);
  font-size: 11px;
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

input[type="number"]::-webkit-outer-spin-button,
input[type="number"]::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}
input[type="number"] {
  -moz-appearance: textfield;
}
.group-header-right { display: flex; align-items: center; gap: 4px; flex-shrink: 0; }
.group-action-btn {
  background: transparent; border: 1px solid transparent; color: var(--text-muted);
  width: 28px; height: 28px; border-radius: 6px; cursor: pointer;
  display: flex; align-items: center; justify-content: center;
  transition: all 0.15s;
}
.group-action-btn:hover { background: color-mix(in srgb, var(--group-color, var(--accent)) 12%, transparent); color: var(--group-color, var(--accent)); border-color: color-mix(in srgb, var(--group-color, var(--accent)) 24%, transparent); transform: translateY(-1px); }
.group-action-btn:active { transform: scale(0.94); }
.group-action-btn i { font-size: 17px; }
.group-body {
  display: grid;
  grid-template-rows: 1fr;
  transition: grid-template-rows 0.28s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.18s ease;
  background: transparent;
  border-radius: 0 0 12px 12px;
  min-height: 0;
}
.group-body > * { min-height: 0; overflow: hidden; }
.group-body.collapsed { grid-template-rows: 0fr; opacity: 0; }
.group-downloads.grab-active:empty {
  min-height: 48px;
  border: 2px dashed var(--border);
  border-radius: 0 0 12px 12px;
  display: flex;
  align-items: center;
  justify-content: center;
}
.group-downloads.grab-active:empty::after {
  content: 'Drop here';
  font-size: 12px;
  color: var(--text-dim);
  font-weight: 500;
}
.group-downloads { padding: 10px 12px 12px; display: flex; flex-direction: column; gap: 6px; }
.compact-group { border-radius: 8px; }
.compact-group .group-drop-overlay { gap: 3px; font-size: 10px; }
.compact-group .group-drop-overlay .material-icons { font-size: 15px; }
.compact-group .group-body { border-radius: 0 0 8px 8px; }
.compact-group .group-header,
.compact-group.group-collapsed .group-header,
.compact-group:not(.group-collapsed) .group-header { padding: 6px 12px; min-height: 46px; gap: 7px; }
.compact-group.group-collapsed .group-header { min-height: 48px; }
.compact-group .group-header-top { gap: 6px; }
.compact-group .group-header-left { gap: 8px; }
.compact-group .group-avatar { width: 28px; height: 28px; border-radius: 8px; }
.compact-group .group-avatar .material-icons { font-size: 16px; }
.compact-group .group-heading { display: flex; align-items: center; gap: 8px; }
.compact-group .group-title-row { flex: 0 1 auto; min-width: 0; gap: 4px; }
.compact-group .group-name { font-size: 12px; max-width: 160px; }
.compact-group .group-toggle-icon { font-size: 15px; }
.compact-group .group-meta { margin-top: 0; gap: 3px; flex-wrap: nowrap; min-width: 0; overflow: hidden; }
.compact-group .group-count, .compact-group .group-size, .compact-group .group-added, .compact-group .group-limit-badge { font-size: 9px; padding: 2px 5px; gap: 3px; }
.compact-group .group-count .material-icons, .compact-group .group-size .material-icons, .compact-group .group-added .material-icons, .compact-group .group-limit-badge .material-icons { font-size: 10px; }
.compact-group .group-header-right { gap: 2px; }
.compact-group .group-action-btn { width: 26px; height: 26px; border-radius: 5px; }
.compact-group .group-action-btn i { font-size: 15px; }
.compact-group .group-progress { left: 8px; right: 8px; height: 2px; }
.compact-group .group-downloads { padding: 4px 8px 8px; gap: 3px; }
.compact-group .group-downloads:empty { padding-top: 0; padding-bottom: 0; }
.compact-group .group-downloads.grab-active:empty { border-radius: 0 0 8px 8px; min-height: 36px; }

/* ── Drag (SortableJS forceFallback) ── */
.drag-list-container {
  position: relative;
}
/* Whole-card grab cursor when reorder mode is on */
.drag-list-container.grab-active .drag-item,
.downloads-list:has(.grab-active) > .drag-item,
.downloads-compact:has(.grab-active) > .drag-item,
.downloads-list.grab-mode .drag-item,
.downloads-compact.grab-mode .drag-item,
.downloads-list.grab-mode .group-container,
.downloads-compact.grab-mode .group-container {
  cursor: grab;
}
.drag-list-container.grab-active .drag-item:active,
.downloads-list.grab-mode .drag-item:active,
.downloads-compact.grab-mode .drag-item:active {
  cursor: grabbing;
}
.drag-item--ghost {
  overflow: hidden;
  opacity: 0;
}
.drag-item--floating {
  opacity: 1 !important;
  filter: none !important;
  pointer-events: none;
  transition: none !important;
  box-shadow: 0 8px 30px rgba(0, 0, 0, 0.25);
  border-radius: 14px;
  z-index: 10000 !important;
}
.drag-item--dragging {
  opacity: 1 !important;
}
.drag-item--chosen {
  /* pressed state while grab engages */
}
.drag-item--settling {
  z-index: 3;
  transform: translate3d(var(--drag-drop-x), var(--drag-drop-y), 0);
  will-change: transform;
}
.drag-item--settling.drag-item--settled {
  transform: translate3d(0, 0, 0);
  transition: transform 300ms ease;
}
.drag-list-move {
  transition: transform 260ms ease;
}
.group-container--ghost {
  overflow: hidden;
  opacity: 0;
}
.group-container--floating {
  opacity: 1 !important;
  filter: none !important;
  pointer-events: none;
  transition: none !important;
  box-shadow: 0 8px 30px rgba(0, 0, 0, 0.3);
  border-radius: 12px;
  z-index: 10000 !important;
}
/* Buttons keep normal cursor even in grab mode */
.drag-item :deep(button),
.drag-item :deep(a),
.drag-item :deep(input),
.drag-item :deep(textarea),
.drag-item :deep(select) {
  cursor: auto;
}

/* â”€â”€ Empty state â”€â”€ */
.empty-state { text-align: center; padding: 60px 20px; color: var(--text-dim); }
.empty-state i { font-size: 48px; opacity: 0.3; }
.empty-state p { font-size: 16px; font-weight: 600; margin: 12px 0 4px; color: var(--text); }
.empty-state span { font-size: 13px; }

/* â”€â”€ Old generic dialog styles (keep for Create Group dialog) â”€â”€ */
.modal-overlay {
  position: fixed; top: 0; left: 0; right: 0; bottom: 0;
  background: rgba(0,0,0,0.85); display: flex; align-items: center; justify-content: center; z-index: 1000;
}
.modal-dialog {
  background: var(--bg-card); border: 1px solid var(--border-strong);
  border-radius: 16px; max-width: 500px; width: 95%;
  box-shadow: 0 8px 24px rgba(0,0,0,0.12);
}
.modal-header { display: flex; align-items: center; gap: 10px; padding: 14px 16px 0; }
.modal-header-left { display: flex; align-items: center; gap: 10px; flex: 1; }
.modal-header-icon { font-size: 22px; color: var(--accent-strong); }
.modal-header h2 { margin: 0; font-size: 16px; font-weight: 700; color: var(--text); }
.modal-close-btn { background: none; border: none; color: var(--text-dim); padding: 4px; cursor: pointer; }
.modal-body { padding: 20px 24px; }
.form-field { display: flex; flex-direction: column; gap: 6px; }
.form-field label { font-size: 12px; font-weight: 600; color: var(--text-muted); }
.form-input {
  width: 100%; height: 38px; background: var(--bg-surface); border: 1px solid var(--border-strong);
  color: var(--text); padding: 0 12px; border-radius: 8px;
  font-size: 13px; font-family: inherit; outline: none; transition: border-color 0.2s ease, box-shadow 0.2s ease;
  box-sizing: border-box;
}
.form-input:focus { border-color: var(--accent-strong); background: rgba(var(--accent-rgb), 0.03); }
.form-input::placeholder { color: var(--text-dim); }
.modal-footer { display: flex; gap: 12px; padding: 20px 24px 24px; border-top: 1px solid var(--border); }
.btn-cancel {
  flex: 1; background: var(--bg-card); color: var(--text);
  border: 1px solid #3a3a4e; padding: 8px 16px; border-radius: 8px;
  font-size: 12px; font-weight: 600; cursor: pointer; transition: all 0.2s;
  display: flex; align-items: center; justify-content: center; gap: 8px;
}
.btn-cancel:hover { background: #3a3a4e; }
.btn-primary {
  flex: 1; background: var(--bg-card); color: var(--text);
  border: 1px solid #3a3a4e; padding: 8px 16px; border-radius: 8px;
  font-size: 12px; font-weight: 600; cursor: pointer; transition: all 0.2s;
  display: flex; align-items: center; justify-content: center; gap: 8px;
}
.btn-primary:hover { background: #3a3a4e; }
.disabled { opacity: 0.5; pointer-events: none; }
.sort-pop-enter-active, .sort-pop-leave-active { transition: all 0.15s ease; }
.sort-pop-enter-from, .sort-pop-leave-to { opacity: 0; transform: translateY(-4px); }

/* =============================================================================
   NEW DOWNLOAD MODAL â€” Beautiful
   ============================================================================= */

/* â”€â”€ Overlay â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ */
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

/* Decorative glow */
.nd-glow {
  position: absolute; top: -60px; left: 50%; transform: translateX(-50%);
  width: 220px; height: 130px;
  background: radial-gradient(ellipse, rgba(var(--accent-rgb), 0.18), transparent 70%);
  pointer-events: none; z-index: 0; filter: blur(20px);
}

/* ── Modal ── */
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

/* â”€â”€ Decorative glow â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ */


/* â”€â”€ Header â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ */
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
.nd-header-left { display: flex; align-items: center; gap: 10px; min-width: 0; flex: 1; }
.nd-avatar {
  width: 36px; height: 36px; border-radius: 10px;
  background: linear-gradient(135deg, rgba(var(--accent-rgb), 0.12), rgba(var(--accent-rgb), 0.04));
  display: flex; align-items: center; justify-content: center;
  flex-shrink: 0; border: 1px solid rgba(var(--accent-rgb), 0.12);
}
.nd-avatar i { font-size: 18px; color: var(--accent-strong); }
.nd-titles { min-width: 0; }
.nd-title { margin: 0; font-size: 14px; font-weight: 700; color: var(--text); letter-spacing: -0.01em; line-height: 1.2; }
.nd-subtitle { font-size: 11px; color: var(--text-muted); font-weight: 400; margin-top: 2px; display: block; }
.nd-header-actions { display: flex; align-items: center; gap: 6px; flex-shrink: 0; }
.nd-icon-btn {
  position: relative; z-index: 1;
  background: none; border: none; color: var(--text-dim);
  width: 28px; height: 28px; border-radius: 7px;
  cursor: pointer; display: flex; align-items: center; justify-content: center;
  transition: all 0.12s ease; flex-shrink: 0; padding: 0;
}
.nd-icon-btn i { font-size: 16px; }
.nd-icon-btn:hover { background: rgba(255,255,255,0.06); color: var(--text); }
.nd-close:hover { background: rgba(239,83,80,0.1); color: var(--status-error); }

.nd-proxy-badge {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 4px 8px;
  border-radius: 100px;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.03em;
  color: var(--accent-strong);
  background: rgba(var(--accent-rgb), 0.07);
  border: 1px solid rgba(var(--accent-rgb), 0.16);
  white-space: nowrap;
}
.nd-proxy-badge.direct {
  color: var(--text-dim);
  background: var(--hover-bg);
  border-color: var(--border);
}
.nd-proxy-dot {
  width: 5px; height: 5px; border-radius: 50%;
  background: var(--accent);
  box-shadow: 0 0 4px rgba(var(--accent-rgb), 0.4);
}
.nd-proxy-badge.direct .nd-proxy-dot {
  background: var(--text-dim);
  box-shadow: none;
}

/* ── Divider ── */
.nd-divider {
  height: 1px;
  background: linear-gradient(90deg, transparent, var(--border-strong), transparent);
  margin: 0 16px;
}

/* ── Body ── */
.nd-body {
  position: relative; z-index: 1;
  flex: 1; overflow-y: auto;
  padding: 12px 16px 8px;
  display: flex; flex-direction: column; gap: 12px;
}

/* Source tabs */
.nd-tabs {
  display: flex; gap: 4px;
  background: var(--hover-bg);
  border-radius: 10px; padding: 4px;
}
.nd-tab {
  flex: 1; display: flex; align-items: center; justify-content: center; gap: 6px;
  height: 32px; border: none; border-radius: 7px;
  background: transparent; color: var(--text-dim);
  font-size: 12px; font-weight: 600; font-family: inherit;
  cursor: pointer; transition: all 0.15s ease;
}
.nd-tab i { font-size: 16px; }
.nd-tab:hover:not(.active) { color: var(--text-muted); background: rgba(255,255,255,0.04); }
.nd-tab.active {
  color: var(--accent-strong);
}

/* Sliding active-tab indicator */
.nd-tabs { position: relative; }
.nd-tab-indicator {
  position: absolute;
  top: 4px; left: 4px;
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
.nd-tab { position: relative; z-index: 1; }

/* Torrent tab extras */
.nd-dropzone {
  display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 4px;
  padding: 18px 12px;
  border: 1.5px dashed var(--border-strong);
  border-radius: 10px;
  background: rgba(255,255,255,0.015);
  cursor: pointer;
  text-align: center;
  transition: all 0.18s ease;
  user-select: none;
}
.nd-dropzone:hover { border-color: rgba(var(--accent-rgb), 0.35); background: rgba(var(--accent-rgb), 0.03); }
.nd-dropzone.dragging {
  border-color: var(--accent-strong);
  background: rgba(var(--accent-rgb), 0.08);
  box-shadow: 0 0 0 3px rgba(var(--accent-rgb), 0.12);
}
.nd-dropzone.filled { border-style: solid; border-color: rgba(var(--accent-rgb), 0.3); background: rgba(var(--accent-rgb), 0.04); }
.nd-dropzone-icon { font-size: 26px; color: var(--text-dim); }
.nd-dropzone.dragging .nd-dropzone-icon,
.nd-dropzone.filled .nd-dropzone-icon { color: var(--accent-strong); }
.nd-dropzone-title { font-size: 12.5px; font-weight: 600; color: var(--text-muted); }
.nd-dropzone-hint { font-size: 11px; color: var(--text-dim); }
.nd-dropzone-name {
  font-size: 12.5px; font-weight: 600; color: var(--accent-strong);
  max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}

/* Torrent contents preview */
.nd-torrent-list {
  display: flex; flex-direction: column; gap: 8px;
}
.nd-torrent-card {
  border: 1px solid var(--border);
  border-radius: 10px;
  overflow: hidden;
  background: rgba(255,255,255,0.015);
}
.nd-torrent-card-head {
  display: flex; align-items: center; gap: 8px;
  padding: 8px 12px;
  background: var(--hover-bg);
  border-bottom: 1px solid var(--border);
  border-radius: 9px 9px 0 0;
}
.nd-torrent-card-head .material-icons {
  font-size: 17px; color: var(--accent-strong); flex-shrink: 0;
}
.nd-torrent-card-name {
  flex: 1; min-width: 0;
  font-size: 12.5px; font-weight: 600; color: var(--text);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.nd-torrent-files {
  border: 1px solid var(--border);
  border-radius: 0;
  overflow: hidden;
  background: rgba(255,255,255,0.015);
}
.nd-torrent-files-loading,
.nd-torrent-files-error {
  display: flex; align-items: center; gap: 6px;
  padding: 10px 12px; font-size: 12px; color: var(--text-dim);
}
.nd-torrent-files-loading .material-icons { font-size: 14px; animation: ndSpinner 1s linear infinite; }
.nd-torrent-files-error { color: var(--status-error); }
.nd-torrent-files-error .material-icons { font-size: 15px; }
.nd-torrent-files-list { display: flex; flex-direction: column; }
.nd-torrent-files-header {
  display: flex; align-items: center; justify-content: space-between;
  padding: 8px 12px; font-size: 11px; font-weight: 700;
  color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.04em;
  background: var(--hover-bg); border-bottom: 1px solid var(--border);
}
.nd-torrent-select-all {
  display: flex; align-items: center; gap: 6px;
  cursor: pointer; font-size: 11px; font-weight: 700;
  color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.04em;
}
.nd-torrent-select-all input[type="checkbox"],
.nd-torrent-file-row input[type="checkbox"] {
  width: 14px; height: 14px;
  accent-color: var(--accent-strong);
  cursor: pointer; flex-shrink: 0;
}
.nd-torrent-files-scroll {
  max-height: 120px; overflow-y: auto;
}
.nd-torrent-file-row {
  display: flex; align-items: center; gap: 8px;
  padding: 6px 12px; font-size: 12px;
  border-bottom: 1px solid var(--border);
  cursor: pointer; transition: background 0.12s;
}
.nd-torrent-file-row:hover { background: rgba(var(--accent-rgb), 0.04); }
.nd-torrent-file-row.unchecked { opacity: 0.5; }
.nd-torrent-file-row:last-child { border-bottom: none; }
.nd-torrent-file-row .material-icons { font-size: 15px; color: var(--text-dim); flex-shrink: 0; }
.nd-torrent-file-name {
  flex: 1; min-width: 0; color: var(--text);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
}
.nd-torrent-file-size { color: var(--text-dim); font-size: 11px; flex-shrink: 0; }

.nd-torrent-hint {
  display: flex; align-items: center; gap: 6px;
  font-size: 11px; color: var(--text-dim); margin: 2px 0 0;
}
.nd-torrent-hint i { font-size: 14px; color: var(--accent-strong); }
.nd-file-chip {
  display: inline-flex; align-items: center; gap: 6px;
  align-self: flex-start; max-width: 100%;
  padding: 4px 6px 4px 10px;
  background: rgba(var(--accent-rgb), 0.05);
  border: 1px solid rgba(var(--accent-rgb), 0.12);
  border-radius: 8px; color: var(--text-muted);
  font-size: 12px;
}
.nd-file-chip i { font-size: 15px; color: var(--accent-strong); }
.nd-chip-name {
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  font-weight: 500; max-width: 220px;
}
.nd-chip-remove {
  background: none; border: none; color: var(--text-dim);
  cursor: pointer; padding: 2px; border-radius: 4px;
  display: flex; align-items: center; justify-content: center;
  transition: color 0.15s, background 0.15s;
}
.nd-chip-remove:hover { color: var(--text); background: var(--hover-bg); }
.nd-chip-remove i { font-size: 14px; color: inherit; }

/* Sections */
.nd-section { display: flex; flex-direction: column; gap: 6px; }
.nd-section-header { display: flex; align-items: center; justify-content: space-between; gap: 8px; }
.nd-section-title {
  font-size: 10px; font-weight: 700; text-transform: uppercase;
  letter-spacing: 0.05em; color: var(--text-dim); margin: 0; opacity: 0.7;
}
.nd-source-badge {
  font-size: 9px; font-weight: 600; color: var(--accent-strong);
  background: rgba(var(--accent-rgb), 0.08); padding: 2px 6px;
  border-radius: 100px; border: 1px solid rgba(var(--accent-rgb), 0.12);
  white-space: nowrap; flex-shrink: 0;
}
/* â”€â”€ Field â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ */ 
.nd-field { display: flex; flex-direction: column; gap: 7px; }
.nd-field-grow { flex: 1; min-width: 0; }
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
.nd-required { color: var(--accent-strong); font-weight: 700; font-size: 13px; line-height: 1; }
.nd-optional { font-weight: 400; color: var(--text-dim); font-size: 11px; }

/* Inputs */
.nd-input-wrap { position: relative; display: flex; align-items: center; }
.nd-input {
  width: 100%; height: 36px;
  background: rgba(255, 255, 255, 0.025);
  border: 1px solid var(--border); color: var(--text);
  padding: 0 10px; border-radius: 8px; font-size: 12px; font-family: inherit;
  outline: none; transition: all 0.15s ease; box-sizing: border-box;
}
.nd-input:hover { border-color: rgba(255,255,255,0.1); background: rgba(255,255,255,0.035); }
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

/* Schedule */
.nd-schedule-section { border: 1px solid var(--border); border-radius: 10px; overflow: hidden; }
.nd-schedule-row {
  display: flex; align-items: center; gap: 10px; padding: 8px 10px;
  cursor: pointer; user-select: none; transition: background 0.12s;
}
.nd-schedule-row:hover { background: var(--hover-bg); }
.nd-schedule-info { flex: 1; min-width: 0; }
.nd-schedule-label { font-size: 12px; font-weight: 500; color: var(--text); display: block; }
.nd-schedule-summary {
  font-size: 10px; color: var(--text-dim); display: block; margin-top: 1px;
  font-weight: 500; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
}
.nd-schedule-summary.pending { color: var(--accent-strong); font-style: italic; }

.nd-category-error {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 10px;
  background: rgba(239, 83, 80, 0.08);
  border: 1px solid rgba(239, 83, 80, 0.2);
  border-radius: 8px;
  font-size: 11px;
  color: var(--status-error);
}

.nd-category-error i {
  font-size: 15px;
}

/* Filename row */
.nd-filename-row { position: relative; }
.nd-filename-wrap { width: 100%; }
.nd-filename-input { padding-right: 34px; }

/* Path input row */
.nd-input-row { display: flex; gap: 6px; align-items: center; }
.nd-input-row .nd-input { flex: 1; }
.nd-path-input { cursor: default; padding-right: 36px; }
.nd-btn-secondary {
  display: inline-flex; align-items: center; gap: 4px;
  background: var(--hover-bg); border: 1px solid var(--border);
  border-radius: 8px; color: var(--text-dim); font-size: 11px; font-weight: 600;
  padding: 8px 10px; cursor: pointer; transition: all 0.12s ease;
  white-space: nowrap; flex-shrink: 0; height: 36px;
}
.nd-btn-secondary i { font-size: 14px; }
.nd-btn-secondary:hover { background: rgba(var(--accent-rgb), 0.08); color: var(--accent-strong); border-color: rgba(var(--accent-rgb), 0.2); }

/* â”€â”€ Duplicate URL Warning â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ */
.nd-duplicate-warning {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  background: rgba(255, 152, 0, 0.08);
  border: 1px solid rgba(255, 152, 0, 0.2);
  border-radius: 10px;
  margin-top: 8px;
}

.nd-duplicate-icon {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  background: rgba(255, 152, 0, 0.15);
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.nd-duplicate-icon i {
  font-size: 20px;
  color: #ff9800;
}

.nd-duplicate-info {
  flex: 1;
  min-width: 0;
}

.nd-duplicate-title {
  font-size: 13px;
  font-weight: 600;
  color: #ffb74d;
  display: block;
}

.nd-duplicate-detail {
  font-size: 11px;
  color: var(--text-dim);
  display: block;
  margin-top: 2px;
}

.nd-dupe-toggle {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 8px;
  background: none;
  border: none;
  padding: 0;
  cursor: pointer;
  color: var(--text);
  font-family: inherit;
}

.nd-dupe-track {
  width: 34px;
  height: 20px;
  border-radius: 10px;
  background: #50360f;
  border: 1px solid rgba(255, 152, 0, 0.25);
  position: relative;
  flex-shrink: 0;
  transition: background 0.15s ease, border-color 0.15s ease;
}

.nd-dupe-track.on {
  background: #c77f1a;
  border-color: #d99a2b;
}

.nd-dupe-thumb {
  position: absolute;
  top: 2px;
  left: 2px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #a9791f;
  transition: transform 0.2s ease, background 0.15s ease;
}

.nd-dupe-track.on .nd-dupe-thumb {
  transform: translateX(14px);
  background: #ffb74d;
}

.nd-duplicate-action {
  display: flex;
  align-items: center;
  gap: 6px;
  background: rgba(255, 152, 0, 0.1);
  border: 1px solid rgba(255, 152, 0, 0.2);
  border-radius: 8px;
  color: #ffb74d;
  padding: 8px 12px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
}

.nd-duplicate-action:hover {
  background: rgba(255, 152, 0, 0.2);
  border-color: rgba(255, 152, 0, 0.3);
}

.nd-duplicate-action i {
  font-size: 16px;
}

/* â”€â”€ Highlight Glow Animation â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ */
.highlight-glow {
  animation: highlightPulse 3s ease-out;
}

@keyframes highlightPulse {
  0% {
    box-shadow: 0 0 0 0 rgba(var(--accent-rgb), 0.7);
    outline: 2px solid rgba(var(--accent-rgb), 0.5);
  }
  50% {
    box-shadow: 0 0 24px 6px rgba(var(--accent-rgb), 0.35);
    outline: 2px solid rgba(var(--accent-rgb), 0.3);
  }
  100% {
    box-shadow: none;
    outline: none;
  }
}

/* â”€â”€ Footer â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ */
.nd-clipboard-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  color: var(--text-dim);
  padding: 3px 8px;
  border-radius: 4px;
  background: var(--hover-bg);
  margin-top: 4px;
}
.nd-clipboard-badge .material-icons { font-size: 13px; }

.nd-detected-row { margin-top: -8px; }
.nd-detected-chip {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  font-size: 12px;
  color: var(--text-dim);
  background: var(--bg-surface);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 5px 10px;
  cursor: pointer;
  transition: all 0.15s ease;
}
.nd-detected-chip:hover {
  border-color: rgba(var(--accent-rgb), 0.3);
  color: var(--text-muted);
  background: rgba(var(--accent-rgb), 0.06);
}
.nd-detected-chip .material-icons { font-size: 14px; color: var(--accent-strong); }
.nd-detected-hint {
  font-size: 10px;
  color: var(--accent-strong);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.nd-more-toggle {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  width: 100%;
  padding: 6px 0;
  background: none;
  border: none;
  border-top: 1px solid var(--border);
  color: var(--text-dim);
  font-size: 12px;
  font-weight: 500;
  font-family: inherit;
  cursor: pointer;
  transition: color 0.15s ease;
}
.nd-more-toggle:hover { color: var(--text-muted); }
.nd-more-toggle .material-icons { font-size: 18px; }

.nd-advanced {
  padding-top: 12px;
}

.nd-footer {
  position: relative; z-index: 1;
  display: flex; gap: 8px; padding: 10px 16px 14px;
  border-top: 1px solid var(--border); flex-shrink: 0;
}
.nd-btn {
  flex: 1; display: inline-flex; align-items: center; justify-content: center;
  gap: 6px; border: none; font-weight: 700; padding: 10px 16px;
  border-radius: 10px; cursor: pointer; font-size: 12.5px;
  transition: all 0.15s ease; font-family: inherit; min-width: 0;
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
.nd-btn-primary:active:not(.disabled) { transform: scale(0.98); box-shadow: 0 1px 4px rgba(var(--accent-rgb), 0.15); }
.nd-btn-secondary {
  background: rgba(255,255,255,0.03); color: var(--text-muted);
  border: 1px solid var(--border);
}
.nd-btn-secondary:hover { background: rgba(var(--accent-rgb), 0.08); color: var(--accent-strong); border-color: rgba(var(--accent-rgb), 0.2); }
.nd-btn-secondary:active { transform: scale(0.98); }

.nd-spinner { animation: ndSpin 0.7s linear infinite; }
@keyframes ndSpin { from { transform: rotate(0deg); } to { transform: rotate(360deg); } }

/* â”€â”€ Responsive â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€ */
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
}

/* â”€â”€ DETAILS MODAL â”€â”€ */
.details-overlay {
  position: fixed; top: 0; left: 0; right: 0; bottom: 0;
  background: rgba(0,0,0,0.55); display: flex; align-items: center; justify-content: center;
  z-index: 1000; backdrop-filter: blur(8px);
}
.details-modal-new {
  background: var(--bg-card); border: 1px solid var(--border); border-radius: 16px;
  box-shadow: 0 4px 16px rgba(0,0,0,0.18), 0 20px 60px rgba(0,0,0,0.25);
  max-width: 640px; width: 94%; max-height: 85vh; display: flex; flex-direction: column;
  overflow: hidden; animation: detailsSlideIn 0.25s cubic-bezier(0.16,1,0.3,1);
}
@keyframes detailsSlideIn {
  from { opacity: 0; transform: translateY(16px) scale(0.95); }
  to { opacity: 1; transform: translateY(0) scale(1); }
}
.details-scale-enter-active { animation: detailsSlideIn 0.25s ease; }
.details-scale-leave-active { animation: detailsSlideIn 0.15s ease reverse; }

.dm-header { display: flex; align-items: center; justify-content: space-between; padding: 16px 20px; border-bottom: 1px solid var(--border); gap: 16px; flex-shrink: 0; }
.dm-header-main { display: flex; align-items: center; gap: 14px; min-width: 0; flex: 1; }
.dm-file-avatar { width: 44px; height: 44px; border-radius: 12px; background: rgba(var(--accent-rgb),0.08); display: flex; align-items: center; justify-content: center; flex-shrink: 0; }
.dm-file-avatar i { font-size: 22px; color: var(--accent-strong); }
.dm-file-info { min-width: 0; }
.dm-filename { margin: 0; font-size: 15px; font-weight: 600; color: var(--text); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; line-height: 1.4; }
.dm-meta-row { display: flex; align-items: center; gap: 6px; margin-top: 3px; flex-wrap: wrap; }
.dm-gid { font-size: 11px; font-family: monospace; color: var(--text-dim); opacity: 0.7; }
.dm-divider { color: var(--text-dim); opacity: 0.4; font-size: 10px; }
.dm-source-badge { display: inline-flex; align-items: center; gap: 4px; font-size: 10.5px; font-weight: 600; padding: 2px 8px; border-radius: 100px; color: var(--accent-strong); background: rgba(var(--accent-rgb),0.08); }
.dm-source-badge i { font-size: 13px; }
.dm-source-badge.yt { color: var(--status-error); background: rgba(239,83,80,0.08); }
.dm-source-badge.http { color: var(--accent-strong); background: rgba(var(--accent-rgb),0.08); }
.dm-header-actions { display: flex; align-items: center; gap: 8px; flex-shrink: 0; }

.dm-status-pill { display: inline-flex; align-items: center; gap: 5px; font-size: 11px; font-weight: 600; padding: 5px 12px; border-radius: 100px; white-space: nowrap; background: var(--hover-bg); color: var(--text-dim); }
.dm-status-pill i { font-size: 15px; }
.dm-status-pill.active { background: rgba(79,195,247,0.14); color: var(--status-active); }
.dm-status-pill.complete { background: rgba(129,199,132,0.14); color: var(--status-complete); }
.dm-status-pill.paused { background: rgba(255,167,38,0.14); color: var(--status-paused); }
.dm-status-pill.error { background: rgba(239,83,80,0.14); color: var(--status-error); }
.dm-status-pill.waiting { background: rgba(100,181,246,0.14); color: var(--status-waiting); }
.dm-status-pill.stopped { background: rgba(105,112,138,0.16); color: #69708a; }
.dm-status-pill.cancelled { background: rgba(158,158,158,0.14); color: var(--status-cancelled); }

.dm-icon-btn { background: none; border: none; color: var(--text-dim); width: 34px; height: 34px; border-radius: 8px; cursor: pointer; display: flex; align-items: center; justify-content: center; transition: all 0.15s ease; flex-shrink: 0; padding: 0; }
.dm-icon-btn i { font-size: 18px; }
.dm-icon-btn:hover { background: var(--hover-bg); color: var(--text); }
.dm-icon-btn.sm { width: 28px; height: 28px; }
.dm-icon-btn.sm i { font-size: 15px; }

.dm-tabs { display: flex; gap: 2px; padding: 0 16px; border-bottom: 1px solid var(--border); overflow-x: auto; flex-shrink: 0; }
.dm-tab { background: none; border: none; border-bottom: 2.5px solid transparent; color: var(--text-dim); padding: 11px 14px; font-size: 12.5px; font-weight: 600; cursor: pointer; display: flex; align-items: center; gap: 7px; white-space: nowrap; transition: all 0.18s ease; }
.dm-tab i { font-size: 17px; }
.dm-tab:hover { color: var(--text); background: rgba(var(--accent-rgb),0.04); }
.dm-tab.active { color: var(--accent-strong); border-bottom-color: var(--accent-strong); }

.dm-body { flex: 1; overflow-y: auto; padding: 16px 20px 12px; min-height: 100px; }
.dm-panel { animation: dmFadeIn 0.2s ease; }
@keyframes dmFadeIn { from { opacity: 0; transform: translateY(4px); } to { opacity: 1; transform: translateY(0); } }
.dm-section { margin-bottom: 20px; }
.dm-section:last-child { margin-bottom: 0; }
.dm-section-title { font-size: 11px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-dim); margin: 0 0 10px; display: flex; align-items: center; gap: 6px; opacity: 0.75; }
.dm-section-title i { font-size: 15px; }

.dm-table { width: 100%; border-collapse: collapse; border-radius: 10px; overflow: hidden; }
.dm-table th, .dm-table td { padding: 10px 14px; text-align: left; font-size: 13px; border-bottom: 1px solid rgba(var(--accent-rgb),0.06); }
.dm-table th { font-weight: 600; color: var(--text-dim); width: 130px; white-space: nowrap; background: var(--hover-bg); }
.dm-table td { color: var(--text); font-weight: 500; background: transparent; }
.dm-table tr:last-child th, .dm-table tr:last-child td { border-bottom: none; }

.dm-status-badge { display: inline-flex; align-items: center; gap: 5px; font-size: 12.5px; font-weight: 600; }
.dm-status-badge i { font-size: 16px; }
.dm-status-badge.active i { color: var(--status-active); }
.dm-status-badge.complete i { color: var(--status-complete); }
.dm-status-badge.paused i { color: var(--status-paused); }
.dm-status-badge.error i { color: var(--status-error); }
.dm-status-badge.waiting i { color: var(--status-waiting); }
.dm-status-badge.stopped i { color: #69708a; }
.dm-status-badge.cancelled i { color: var(--status-cancelled); }

.dm-progress-cell { min-width: 140px; }
.dm-progress-wrap { display: flex; align-items: center; gap: 10px; }
.dm-progress-track { flex: 1; max-width: 160px; height: 6px; background: rgba(var(--accent-rgb),0.1); border-radius: 3px; overflow: hidden; }
.dm-progress-fill { height: 100%; background: linear-gradient(90deg, var(--accent), rgba(var(--accent-rgb),0.7)); border-radius: 3px; transition: width 0.4s ease; }
.dm-progress-pct { font-size: 13px; font-weight: 600; color: var(--text); min-width: 38px; }
.dm-speed-cell { color: var(--accent-strong); font-weight: 600; }

.dm-chip { display: inline-flex; align-items: center; gap: 4px; padding: 3px 10px; background: rgba(var(--accent-rgb),0.08); color: var(--accent-strong); border-radius: 100px; font-size: 12px; font-weight: 600; line-height: 1.4; }
.dm-chip i { font-size: 14px; }
.dm-priority { font-size: 12px; font-weight: 600; padding: 3px 10px; border-radius: 100px; }
.dm-priority.normal { background: rgba(105,112,138,0.08); color: var(--text-dim); }
.dm-source-tag { font-size: 12px; font-weight: 600; padding: 3px 10px; border-radius: 100px; }
.dm-source-tag.yt { background: rgba(239,83,80,0.08); color: var(--status-error); }
.dm-source-tag.http { background: rgba(var(--accent-rgb),0.08); color: var(--accent-strong); }
.dm-source-tag.direct { background: var(--hover-bg); color: var(--text-dim); }

.dm-url-cell { display: flex; align-items: center; gap: 8px; }
.dm-url-text { flex: 1; word-break: break-all; font-size: 12px; color: var(--text-dim); min-width: 0; }
.dm-path-cell { word-break: break-all; font-size: 12px; color: var(--text-dim); display: flex; align-items: center; gap: 8px; }

.dm-file-table-wrapper { border-radius: 10px; overflow: hidden; border: 1px solid var(--border); }
.dm-file-table { width: 100%; border-collapse: collapse; }
.dm-file-table thead { background: var(--hover-bg); }
.dm-file-table th { padding: 9px 12px; font-size: 11px; font-weight: 700; color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.04em; text-align: left; border-bottom: 1px solid var(--border); }
.dm-file-table td { padding: 10px 12px; font-size: 12.5px; color: var(--text); border-bottom: 1px solid rgba(var(--accent-rgb),0.05); }
.dm-file-table tr:last-child td { border-bottom: none; }
.dm-file-name-cell { display: flex; align-items: center; gap: 8px; min-width: 0; }
.dm-file-name-cell i { font-size: 18px; color: var(--accent-strong); flex-shrink: 0; }
.dm-file-name { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; font-weight: 600; }
.dm-file-progress-cell { min-width: 120px; }

.dm-mini-progress { display: flex; align-items: center; gap: 8px; }
.dm-mini-track { width: 60px; height: 4px; background: rgba(var(--accent-rgb),0.1); border-radius: 2px; overflow: hidden; }
.dm-mini-fill { height: 100%; background: var(--accent); border-radius: 2px; transition: width 0.3s; }
.dm-mini-progress span { font-size: 11.5px; color: var(--text-dim); }

.dm-empty-state { text-align: center; padding: 40px 20px; }
.dm-empty-state i { font-size: 40px; color: var(--text-dim); opacity: 0.5; margin-bottom: 12px; }
.dm-empty-state p { color: var(--text-dim); font-size: 14px; font-weight: 500; margin: 0 0 4px; }
.dm-hint { font-size: 12px; color: var(--text-dim); opacity: 0.6; }

.dm-source-list { list-style: none; padding: 0; margin: 0; }
.dm-source-list li { display: flex; align-items: center; gap: 8px; padding: 8px 12px; background: var(--hover-bg); border-radius: 8px; margin-bottom: 4px; }
.dm-source-item { flex: 1; font-size: 12px; color: var(--text-dim); word-break: break-all; min-width: 0; }

.dm-error-section .dm-section-title i { color: var(--status-error); }
.dm-error-box { background: rgba(239,83,80,0.05); border: 1px solid rgba(239,83,80,0.12); border-radius: 10px; padding: 12px 14px; }
.dm-error-box pre { margin: 0 0 10px; font-size: 12px; color: var(--status-error); white-space: pre-wrap; word-break: break-word; font-family: inherit; line-height: 1.5; }
.dm-btn-secondary { display: inline-flex; align-items: center; gap: 6px; background: rgba(239,83,80,0.1); color: var(--status-error); border: 1px solid rgba(239,83,80,0.1); font-weight: 600; padding: 6px 14px; border-radius: 6px; cursor: pointer; font-size: 12px; transition: all 0.15s; }
.dm-btn-secondary:hover { background: rgba(239,83,80,0.18); }
.dm-btn-secondary i { font-size: 15px; }

.dm-proxy-actions { display: flex; align-items: center; gap: 10px; margin-top: 10px; }
.dm-proxy-save {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background: rgba(var(--accent-rgb), 0.12);
  color: var(--accent-strong);
  border: 1px solid rgba(var(--accent-rgb), 0.25);
  font-weight: 600;
  padding: 6px 14px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.15s;
}
.dm-proxy-save:hover { background: rgba(var(--accent-rgb), 0.2); }
.dm-proxy-save:disabled { opacity: 0.5; cursor: not-allowed; }
.dm-proxy-save i { font-size: 15px; }
.dm-proxy-message { font-size: 12px; }
.dm-proxy-message.ok { color: var(--status-complete); }
.dm-proxy-message:not(.ok) { color: var(--status-error); }

.dm-footer { display: flex; justify-content: flex-end; padding: 12px 20px; border-top: 1px solid var(--border); flex-shrink: 0; }
.dm-btn { display: inline-flex; align-items: center; gap: 6px; background: var(--accent); color: #0a0e1a; border: none; font-weight: 700; padding: 10px 22px; border-radius: 10px; cursor: pointer; font-size: 13px; transition: all 0.15s ease; box-shadow: 0 2px 8px rgba(var(--accent-rgb),0.2); }
.dm-btn i { font-size: 16px; }
.dm-btn:hover { transform: translateY(-1px); box-shadow: 0 4px 16px rgba(var(--accent-rgb),0.35); }
.dm-btn:active { transform: scale(0.97); }
.modal-fade-enter-active, .modal-fade-leave-active { transition: opacity 0.2s ease; }
.modal-fade-enter-from, .modal-fade-leave-to { opacity: 0; }

.source-stack { display: flex; flex-direction: column; gap: 14px; }

/* â”€â”€ Scroll to top â”€â”€ */
.scroll-top-btn {
  position: fixed; bottom: 24px; right: 24px; z-index: 500;
  width: 44px; height: 44px; border-radius: 50%;
  background: var(--bg-card); border: 1px solid var(--border-strong);
  color: var(--text); cursor: pointer; display: flex; align-items: center;
  justify-content: center; box-shadow: var(--shadow-lg);
  transition: all 0.2s ease;
}
.scroll-top-btn:hover { transform: translateY(-2px); background: var(--accent); color: #0a0e1a; border-color: var(--accent-strong); }
.scroll-top-btn:active { transform: scale(0.95); }
.scroll-top-btn .material-icons { font-size: 24px; }

/* Speed toggle */
.speed-toggle-row {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 10px;
}
.speed-toggle {
  width: 36px;
  height: 20px;
  border-radius: 10px;
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
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--text-dim);
  transition: transform 0.25s cubic-bezier(0.34, 1.4, 0.64, 1), background 0.25s ease;
}
.speed-toggle.on .speed-toggle-knob {
  transform: translateX(16px);
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

/* Speed slide transition */
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

/* Stepper (number input with +/- buttons) */
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
  background: var(--bg-surface);
  color: var(--text);
  text-align: center;
  font-weight: 600;
  font-size: 13px;
}
.stepper input:focus {
  outline: none;
  box-shadow: none;
}
</style>







