pub const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Web Remote</title>
  <style>
    :root {
      --bg-dark: #000000;
      --bg-card: #0F0F23;
      --bg-surface: #1E1B4B;
      --accent: #E11D48;
      --accent-hover: #F43F5E;
      --text-main: #F8FAFC;
      --text-muted: #94A3B8;
      --border: rgba(255, 255, 255, 0.12);
      --glass: rgba(15, 15, 35, 0.85);
    }
    * { box-sizing: border-box; margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; }
    body { background: var(--bg-dark); color: var(--text-main); min-height: 100vh; display: flex; flex-direction: column; overflow-x: hidden; }
    
    header { background: var(--glass); backdrop-filter: blur(12px); border-bottom: 1px solid var(--border); position: sticky; top: 0; z-index: 50; padding: 0.75rem 1.5rem; display: flex; justify-content: space-between; align-items: center; gap: 1rem; flex-wrap: wrap; }
    .brand { font-size: 1.25rem; font-weight: 700; display: flex; align-items: center; gap: 0.5rem; white-space: nowrap; }
    .status-badge { font-size: 0.75rem; padding: 0.25rem 0.6rem; border-radius: 9999px; background: rgba(34, 197, 94, 0.2); color: #4ADE80; display: inline-flex; align-items: center; gap: 0.35rem; font-weight: 600; white-space: nowrap; }
    .status-badge.offline { background: rgba(239, 68, 68, 0.2); color: #F87171; }

    .nav-tabs { display: flex; gap: 0.4rem; background: rgba(255, 255, 255, 0.05); padding: 0.25rem; border-radius: 0.75rem; border: 1px solid var(--border); }
    .nav-tab { background: transparent; border: none; color: var(--text-muted); padding: 0.45rem 0.9rem; border-radius: 0.5rem; font-size: 0.875rem; font-weight: 600; cursor: pointer; display: flex; align-items: center; gap: 0.4rem; transition: all 0.2s; }
    .nav-tab:hover { color: var(--text-main); background: rgba(255, 255, 255, 0.08); }
    .nav-tab.active { color: #fff; background: var(--accent); }

    main { flex: 1; padding: 1.5rem; max-width: 1100px; margin: 0 auto; width: 100%; display: flex; flex-direction: column; }

    .view-panel { width: 100%; }
    #view-remote { max-width: 650px; margin: auto; display: flex; align-items: center; justify-content: center; }

    .remote-card { background: var(--bg-card); border: 1px solid var(--border); border-radius: 1.25rem; padding: 1.75rem; backdrop-filter: blur(12px); width: 100%; box-shadow: 0 20px 40px rgba(0,0,0,0.5); }
    .now-playing { text-align: center; }
    
    .video-frame-wrap { width: 100%; aspect-ratio: 16/9; background: #080810; border-radius: 0.85rem; border: 1px solid var(--border); display: flex; align-items: center; justify-content: center; overflow: hidden; margin-bottom: 1.25rem; position: relative; }
    .video-frame-img { width: 100%; height: 100%; object-fit: cover; }
    .frame-placeholder { color: var(--text-muted); font-size: 0.95rem; display: flex; flex-direction: column; align-items: center; gap: 0.5rem; }
    
    .now-title { font-size: 1.15rem; font-weight: 700; margin-bottom: 0.35rem; word-break: break-all; color: var(--text-main); }
    .now-time { font-size: 0.9rem; color: var(--text-muted); font-family: monospace; }
    
    .seek-container { margin: 1.5rem 0 1rem 0; display: flex; align-items: center; gap: 0.75rem; }
    .seek-bar { flex: 1; accent-color: var(--accent); height: 6px; cursor: pointer; border-radius: 3px; }
    
    .control-row { display: flex; justify-content: center; align-items: center; gap: 1.5rem; margin: 1.25rem 0; }
    .ctrl-btn { background: var(--bg-surface); border: 1px solid var(--border); color: var(--text-main); width: 50px; height: 50px; border-radius: 50%; display: flex; align-items: center; justify-content: center; cursor: pointer; transition: all 0.2s; }
    .ctrl-btn:hover { background: rgba(255,255,255,0.15); transform: scale(1.08); }
    .ctrl-btn.play-btn { width: 64px; height: 64px; background: var(--accent); border: none; box-shadow: 0 8px 20px rgba(225, 29, 72, 0.4); }
    .ctrl-btn.play-btn:hover { background: var(--accent-hover); transform: scale(1.08); }

    .vol-container { display: flex; align-items: center; gap: 0.75rem; margin-top: 1.25rem; background: rgba(255,255,255,0.03); padding: 0.75rem 1.1rem; border-radius: 0.85rem; border: 1px solid var(--border); }
    .vol-bar { flex: 1; accent-color: var(--accent); height: 5px; cursor: pointer; }

    /* Explorer View */
    .explorer-card { background: var(--bg-card); border: 1px solid var(--border); border-radius: 1.25rem; padding: 1.25rem; backdrop-filter: blur(12px); width: 100%; box-shadow: 0 20px 40px rgba(0,0,0,0.5); }
    .explorer-header { background: rgba(255, 255, 255, 0.02); border: 1px solid var(--border); border-radius: 0.85rem; padding: 0.85rem 1rem; margin-bottom: 1.25rem; display: flex; flex-direction: column; gap: 0.75rem; }
    .explorer-nav-row { display: flex; align-items: center; gap: 0.6rem; min-width: 0; }
    .nav-btn { background: var(--bg-surface); border: 1px solid var(--border); color: var(--text-main); padding: 0.45rem 0.85rem; border-radius: 0.5rem; cursor: pointer; display: flex; align-items: center; gap: 0.35rem; font-size: 0.85rem; font-weight: 600; white-space: nowrap; transition: all 0.2s; }
    .nav-btn:hover:not(:disabled) { background: rgba(255, 255, 255, 0.15); transform: translateY(-1px); }
    .nav-btn:disabled { opacity: 0.4; cursor: not-allowed; }
    
    .breadcrumbs-scroll { flex: 1; overflow-x: auto; white-space: nowrap; padding: 0.2rem 0; scrollbar-width: thin; }
    .breadcrumbs { display: flex; align-items: center; font-family: monospace; font-size: 0.9rem; }
    .crumb { cursor: pointer; color: var(--text-muted); padding: 0.2rem 0.4rem; border-radius: 0.35rem; transition: all 0.15s; }
    .crumb:hover { color: var(--text-main); background: rgba(255, 255, 255, 0.1); }
    .crumb.active { color: var(--text-main); font-weight: 700; }
    .crumb-sep { color: var(--border); user-select: none; }

    .explorer-filter-row { display: flex; align-items: center; justify-content: space-between; gap: 1rem; }
    .search-input-wrap { flex: 1; position: relative; display: flex; align-items: center; }
    .search-icon { position: absolute; left: 0.75rem; width: 16px; height: 16px; fill: var(--text-muted); pointer-events: none; }
    .search-filter { width: 100%; background: rgba(0, 0, 0, 0.4); border: 1px solid var(--border); border-radius: 0.5rem; padding: 0.45rem 0.75rem 0.45rem 2.2rem; color: var(--text-main); font-size: 0.85rem; outline: none; transition: border 0.2s; }
    .search-filter:focus { border-color: var(--accent); }
    .explorer-meta { font-size: 0.8rem; color: var(--text-muted); white-space: nowrap; }

    .explorer-loading { text-align: center; padding: 2.5rem 1rem; color: var(--text-muted); font-size: 0.95rem; }
    .spinner { width: 32px; height: 32px; border: 3px solid rgba(255, 255, 255, 0.1); border-top-color: var(--accent); border-radius: 50%; animation: spin 0.8s linear infinite; margin: 0 auto 0.75rem auto; }
    @keyframes spin { to { transform: rotate(360deg); } }

    .explorer-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 1rem; }
    .grid-item { background: rgba(255, 255, 255, 0.02); border: 1px solid var(--border); border-radius: 0.85rem; overflow: hidden; cursor: pointer; display: flex; flex-direction: column; transition: all 0.2s cubic-bezier(0.16, 1, 0.3, 1); }
    .grid-item:hover { transform: translateY(-3px); box-shadow: 0 10px 25px rgba(0, 0, 0, 0.4); }
    .grid-item.media-item:hover { border-color: var(--accent); }
    .grid-item.folder-item:hover { border-color: #38BDF8; }

    .item-thumb { width: 100%; aspect-ratio: 16/10; background: #080812; display: flex; align-items: center; justify-content: center; position: relative; overflow: hidden; }
    .folder-item .item-thumb svg { fill: #38BDF8; width: 44px; height: 44px; }
    .file-item .item-thumb svg { fill: var(--text-muted); width: 38px; height: 38px; }
    .thumb-img { width: 100%; height: 100%; object-fit: cover; transition: transform 0.2s; }
    .grid-item:hover .thumb-img { transform: scale(1.05); }
    .fallback-icon { width: 100%; height: 100%; display: flex; align-items: center; justify-content: center; background: #111124; }
    .fallback-icon svg { fill: var(--accent); width: 40px; height: 40px; }

    .play-hover-badge { position: absolute; width: 44px; height: 44px; border-radius: 50%; background: var(--accent); display: flex; align-items: center; justify-content: center; box-shadow: 0 4px 15px rgba(225, 29, 72, 0.6); opacity: 0; transform: scale(0.8); transition: all 0.2s; pointer-events: none; }
    .play-hover-badge svg { width: 22px; height: 22px; fill: #fff; transform: translateX(1px); }
    .grid-item.media-item:hover .play-hover-badge { opacity: 1; transform: scale(1); }

    .item-info { padding: 0.75rem; display: flex; flex-direction: column; gap: 0.25rem; }
    .item-name { font-size: 0.85rem; font-weight: 600; color: var(--text-main); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .item-meta { font-size: 0.75rem; color: var(--text-muted); }
    .explorer-empty { grid-column: 1 / -1; text-align: center; padding: 3rem 1rem; color: var(--text-muted); }

    .toast { position: fixed; bottom: 2rem; left: 50%; transform: translateX(-50%) translateY(20px); background: rgba(15, 15, 35, 0.95); border: 1px solid var(--accent); padding: 0.75rem 1.5rem; border-radius: 9999px; color: #fff; font-size: 0.9rem; font-weight: 500; box-shadow: 0 10px 30px rgba(0,0,0,0.6); opacity: 0; pointer-events: none; transition: all 0.3s cubic-bezier(0.16, 1, 0.3, 1); z-index: 1000; }
    .toast.show { opacity: 1; transform: translateX(-50%) translateY(0); }

    svg { width: 22px; height: 22px; fill: currentColor; }

    @media (max-width: 640px) {
      header { padding: 0.75rem 1rem; gap: 0.5rem; }
      .brand span { display: none; }
      .nav-tab { padding: 0.4rem 0.65rem; font-size: 0.8rem; }
      .explorer-grid { grid-template-columns: repeat(auto-fill, minmax(135px, 1fr)); gap: 0.65rem; }
    }
  </style>
</head>
<body>
  <header>
    <div class="brand">
      <svg viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 14.5v-9l6 4.5-6 4.5z"/></svg>
      <span id="app-brand">Application</span>
    </div>
    
    <nav class="nav-tabs">
      <button class="nav-tab active" id="tab-btn-remote" onclick="switchTab('remote')">
        <svg viewBox="0 0 24 24" style="width:16px;height:16px;"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 14.5v-9l6 4.5-6 4.5z"/></svg>
        <span>Remote Control</span>
      </button>
      <button class="nav-tab" id="tab-btn-explorer" onclick="switchTab('explorer')">
        <svg viewBox="0 0 24 24" style="width:16px;height:16px;"><path d="M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z"/></svg>
        <span>Media Explorer</span>
      </button>
    </nav>

    <div class="status-badge" id="net-status">
      <span>●</span> Initializing…
    </div>
  </header>

  <main>
    <!-- View 1: Remote Control -->
    <div id="view-remote" class="view-panel">
      <div class="remote-card">
        <div class="now-playing">
          <div class="video-frame-wrap" id="frame-box">
            <div class="frame-placeholder">
              <svg viewBox="0 0 24 24" style="width: 48px; height: 48px; opacity: 0.5;"><path d="M21 3H3c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h18c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H3V5h18v14zM8 15c.55 0 1-.45 1-1s-.45-1-1-1-1 .45-1 1 .45 1 1 1zm4 0c.55 0 1-.45 1-1s-.45-1-1-1-1 .45-1 1 .45 1 1 1zm4 0c.55 0 1-.45 1-1s-.45-1-1-1-1 .45-1 1 .45 1 1 1z"/></svg>
              <div>No Video Loaded</div>
              <button class="nav-btn" style="margin-top: 0.5rem;" onclick="switchTab('explorer')">Open Media Explorer</button>
            </div>
          </div>
          <div class="now-title" id="lbl-title">Initializing…</div>
          <div class="now-time" id="lbl-time">— / —</div>
        </div>

        <div class="seek-container">
          <input type="range" class="seek-bar" id="seek-slider" min="0" max="100" value="0" onchange="onSeek(this.value)">
        </div>

        <div class="control-row">
          <button class="ctrl-btn" title="Seek -10s" onclick="sendCmd('seek', {seconds: -10})">
            <svg viewBox="0 0 24 24"><path d="M11 18V6l-8.5 6 8.5 6zm.5-6l8.5 6V6l-8.5 6z"/></svg>
          </button>
          <button class="ctrl-btn play-btn" id="btn-play" title="Play / Pause" onclick="sendCmd('toggle_pause')">
            <svg id="icon-play" viewBox="0 0 24 24" style="width: 28px; height: 28px;"><path d="M8 5v14l11-7z"/></svg>
          </button>
          <button class="ctrl-btn" title="Seek +10s" onclick="sendCmd('seek', {seconds: 10})">
            <svg viewBox="0 0 24 24"><path d="M4 18l8.5-6L4 6v12zm9-12v12l8.5-6L13 6z"/></svg>
          </button>
        </div>

        <div class="vol-container">
          <svg viewBox="0 0 24 24"><path d="M3 9v6h4l5 5V4L7 9H3zm13.5 3c0-1.77-1.02-3.29-2.5-4.03v8.05c1.48-.73 2.5-2.25 2.5-4.02z"/></svg>
          <input type="range" class="vol-bar" id="vol-slider" min="0" max="130" value="0" disabled oninput="onVol(this.value)">
          <span id="lbl-vol" style="font-size: 0.85rem; width: 45px; text-align: right; font-family: monospace;">—</span>
        </div>
      </div>
    </div>

    <!-- View 2: Media Explorer -->
    <div id="view-explorer" class="view-panel" style="display: none;">
      <div class="explorer-card">
        <div class="explorer-header">
          <div class="explorer-nav-row">
            <button class="nav-btn" id="btn-up" onclick="goParent()" title="Go to Parent Directory">
              <svg viewBox="0 0 24 24"><path d="M4 12l1.41 1.41L11 7.83V20h2V7.83l5.58 5.59L20 12l-8-8-8 8z"/></svg>
              <span>.. Up</span>
            </button>
            <button class="nav-btn" onclick="fetchDirectory(currentBrowsePath)" title="Reload">
              <svg viewBox="0 0 24 24"><path d="M17.65 6.35C16.2 4.9 14.21 4 12 4c-4.42 0-7.99 3.58-7.99 8s3.57 8 7.99 8c3.73 0 6.84-2.55 7.73-6h-2.08c-.82 2.33-3.04 4-5.65 4-3.31 0-6-2.69-6-6s2.69-6 6-6c1.66 0 3.14.69 4.22 1.78L13 11h7V4l-2.35 2.35z"/></svg>
            </button>
            <div class="breadcrumbs-scroll">
              <div class="breadcrumbs" id="breadcrumbs"></div>
            </div>
          </div>
          <div class="explorer-filter-row">
            <div class="search-input-wrap">
              <svg class="search-icon" viewBox="0 0 24 24"><path d="M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z"/></svg>
              <input type="text" id="search-filter" class="search-filter" placeholder="Search files or folders..." oninput="onFilterChange(this.value)">
            </div>
            <div class="explorer-meta" id="explorer-count">0 items</div>
          </div>
        </div>

        <div id="explorer-loading" class="explorer-loading" style="display: none;">
          <div class="spinner"></div>
          <div>Loading directory...</div>
        </div>

        <div class="explorer-grid" id="explorer-items"></div>
      </div>
    </div>
  </main>

  <div id="toast" class="toast"></div>

  <script>
    let ws;
    let isPlayingState = false;
    let lastFrameUpdate = 0;

    let currentBrowsePath = '';
    let parentBrowsePath = null;
    let currentEntries = [];
    let searchQuery = '';
    let toastTimer = null;

    async function initWS() {
      const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
      let runtime;
      try {
        const response = await fetch('/api/runtime/config');
        if (!response.ok) return;
        runtime = await response.json();
        document.getElementById('app-brand').textContent = runtime.appName;
        document.title = `${runtime.appName} Web Remote`;
        document.documentElement.lang = runtime.locale;
        document.documentElement.dir = runtime.direction;
      } catch (_) {
        setTimeout(initWS, 3000);
        return;
      }
      const wsUrl = `${proto}//${location.hostname}:${runtime.wsPort}`;
      
      try {
        ws = new WebSocket(wsUrl);
        
        ws.onopen = () => {
          document.getElementById('net-status').classList.remove('offline');
          document.getElementById('net-status').innerHTML = '<span>●</span> WS Connected';
        };
        
        ws.onclose = () => {
          document.getElementById('net-status').classList.add('offline');
          document.getElementById('net-status').innerHTML = '<span>●</span> HTTP Polling';
          setTimeout(initWS, 3000);
        };

        ws.onmessage = (ev) => {
          try {
            const state = JSON.parse(ev.data);
            updateRemoteUI(state);
          } catch(e) {}
        };
      } catch(e) {}

      // REST Polling Fallback (ensures status updates work seamlessly everywhere)
      setInterval(async () => {
        try {
          const res = await fetch('/api/player/status');
          if (res.ok) {
            const state = await res.json();
            updateRemoteUI(state);
            if (!ws || ws.readyState !== WebSocket.OPEN) {
              document.getElementById('net-status').classList.remove('offline');
              document.getElementById('net-status').innerHTML = '<span>●</span> Connected';
            }
          }
        } catch(e) {}
      }, 500);

      // Auto-refresh video frame snapshot during active playback
      setInterval(() => {
        if (isPlayingState) {
          refreshVideoFrame();
        }
      }, 1000);
    }

    function sendCmd(cmd, payload = {}) {
      const msg = JSON.stringify({ command: cmd, ...payload });
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(msg);
      }
      fetch('/api/player/command', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: msg
      }).catch(() => {});
    }

    function switchTab(tab) {
      const remoteView = document.getElementById('view-remote');
      const explorerView = document.getElementById('view-explorer');
      const tabRemote = document.getElementById('tab-btn-remote');
      const tabExplorer = document.getElementById('tab-btn-explorer');

      if (tab === 'explorer') {
        remoteView.style.display = 'none';
        explorerView.style.display = 'block';
        tabRemote.classList.remove('active');
        tabExplorer.classList.add('active');
        if (!currentBrowsePath) {
          fetchDirectory();
        }
      } else {
        explorerView.style.display = 'none';
        remoteView.style.display = 'flex';
        tabExplorer.classList.remove('active');
        tabRemote.classList.add('active');
      }
    }

    function showToast(msg) {
      const toast = document.getElementById('toast');
      if (!toast) return;
      toast.textContent = msg;
      toast.classList.add('show');
      if (toastTimer) clearTimeout(toastTimer);
      toastTimer = setTimeout(() => {
        toast.classList.remove('show');
      }, 2500);
    }

    async function fetchDirectory(path) {
      const loadingEl = document.getElementById('explorer-loading');
      const itemsEl = document.getElementById('explorer-items');
      if (loadingEl) loadingEl.style.display = 'block';
      if (itemsEl) itemsEl.style.opacity = '0.5';

      try {
        const url = path ? `/api/fs/browse?path=${encodeURIComponent(path)}` : '/api/fs/browse';
        const res = await fetch(url);
        if (res.ok) {
          const data = await res.json();
          currentBrowsePath = data.current_path || '';
          parentBrowsePath = data.parent_path;
          currentEntries = data.entries || [];
          renderExplorer();
        } else {
          showToast('Failed to load directory');
        }
      } catch(e) {
        showToast('Error browsing directory');
      } finally {
        if (loadingEl) loadingEl.style.display = 'none';
        if (itemsEl) itemsEl.style.opacity = '1';
      }
    }

    function goParent() {
      if (parentBrowsePath) {
        fetchDirectory(parentBrowsePath);
      }
    }

    function onFilterChange(val) {
      searchQuery = val || '';
      renderExplorer();
    }

    function onMediaClick(path, name) {
      sendCmd('open', { target: path, path: path });
      showToast('Playing: ' + name);
      switchTab('remote');
    }

    function formatSize(bytes) {
      if (!bytes || bytes === 0) return '';
      const units = ['B', 'KB', 'MB', 'GB', 'TB'];
      let b = bytes;
      let i = 0;
      while (b >= 1024 && i < units.length - 1) {
        b /= 1024;
        i++;
      }
      return b.toFixed(1) + ' ' + units[i];
    }

    function renderBreadcrumbs(pathStr) {
      const container = document.getElementById('breadcrumbs');
      if (!container) return;
      container.innerHTML = '';
      if (!pathStr) return;

      const isWindows = pathStr.includes('\\');
      const sep = isWindows ? '\\' : '/';
      const parts = pathStr.split(sep).filter(p => p.length > 0);

      // Root crumb
      const rootSpan = document.createElement('span');
      rootSpan.className = 'crumb';
      rootSpan.textContent = isWindows ? (parts[0] ? parts[0] + '\\' : '\\') : '/';
      const rootPath = isWindows ? (parts[0] ? parts[0] + '\\' : '\\') : '/';
      rootSpan.onclick = () => fetchDirectory(rootPath);
      container.appendChild(rootSpan);

      let accumulated = isWindows ? (parts[0] ? parts[0] : '') : '';
      const startIdx = isWindows ? 1 : 0;

      for (let i = startIdx; i < parts.length; i++) {
        const part = parts[i];
        const sepSpan = document.createElement('span');
        sepSpan.className = 'crumb-sep';
        sepSpan.textContent = ' / ';
        container.appendChild(sepSpan);

        if (isWindows) {
          accumulated += '\\' + part;
        } else {
          accumulated += '/' + part;
        }

        const crumbPath = accumulated;
        const crumbSpan = document.createElement('span');
        crumbSpan.className = 'crumb' + (i === parts.length - 1 ? ' active' : '');
        crumbSpan.textContent = part;
        crumbSpan.onclick = () => fetchDirectory(crumbPath);
        container.appendChild(crumbSpan);
      }
    }

    function renderExplorer() {
      const container = document.getElementById('explorer-items');
      const countLabel = document.getElementById('explorer-count');
      if (!container) return;
      container.innerHTML = '';

      renderBreadcrumbs(currentBrowsePath);

      const upBtn = document.getElementById('btn-up');
      if (upBtn) {
        upBtn.disabled = !parentBrowsePath;
      }

      const q = searchQuery.toLowerCase().trim();
      const entries = q ? currentEntries.filter(e => e.name.toLowerCase().includes(q)) : currentEntries;

      if (countLabel) {
        countLabel.textContent = `${entries.length} item${entries.length === 1 ? '' : 's'}`;
      }

      if (entries.length === 0) {
        const emptyEl = document.createElement('div');
        emptyEl.className = 'explorer-empty';
        emptyEl.innerHTML = `
          <svg viewBox="0 0 24 24" style="width:48px;height:48px;opacity:0.4;margin-bottom:0.5rem;"><path d="M19 5v14H5V5h14m0-2H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2z"/></svg>
          <div>No items found</div>
        `;
        container.appendChild(emptyEl);
        return;
      }

      entries.forEach(item => {
        const card = document.createElement('div');
        card.className = 'grid-item ' + (item.is_dir ? 'folder-item' : (item.is_media ? 'media-item' : 'file-item'));

        const thumbWrap = document.createElement('div');
        thumbWrap.className = 'item-thumb';

        if (item.is_dir) {
          thumbWrap.innerHTML = '<svg viewBox="0 0 24 24"><path d="M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z"/></svg>';
          card.onclick = () => fetchDirectory(item.path);
        } else if (item.is_media) {
          const img = document.createElement('img');
          img.className = 'thumb-img';
          img.loading = 'lazy';
          img.src = `/api/fs/thumbnail?path=${encodeURIComponent(item.path)}`;

          const fallback = document.createElement('div');
          fallback.className = 'fallback-icon';
          fallback.style.display = 'none';
          fallback.innerHTML = '<svg viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 14.5v-9l6 4.5-6 4.5z"/></svg>';

          img.onerror = () => {
            img.style.display = 'none';
            fallback.style.display = 'flex';
          };

          thumbWrap.appendChild(img);
          thumbWrap.appendChild(fallback);

          const badge = document.createElement('span');
          badge.className = 'play-hover-badge';
          badge.innerHTML = '<svg viewBox="0 0 24 24"><path d="M8 5v14l11-7z"/></svg>';
          thumbWrap.appendChild(badge);

          card.onclick = () => onMediaClick(item.path, item.name);
        } else {
          thumbWrap.innerHTML = '<svg viewBox="0 0 24 24"><path d="M14 2H6c-1.1 0-1.99.9-1.99 2L4 20c0 1.1.89 2 1.99 2H18c1.1 0 2-.9 2-2V8l-6-6zm2 16H8v-2h8v2zm0-4H8v-2h8v2zm-3-5V3.5L18.5 9H13z"/></svg>';
        }

        const info = document.createElement('div');
        info.className = 'item-info';

        const nameEl = document.createElement('div');
        nameEl.className = 'item-name';
        nameEl.textContent = item.name;
        nameEl.title = item.name;

        const metaEl = document.createElement('div');
        metaEl.className = 'item-meta';
        metaEl.textContent = item.is_dir ? 'Folder' : (item.is_media ? (formatSize(item.size_bytes) || 'Media') : formatSize(item.size_bytes));

        info.appendChild(nameEl);
        info.appendChild(metaEl);

        card.appendChild(thumbWrap);
        card.appendChild(info);
        container.appendChild(card);
      });
    }

    function updateRemoteUI(s) {
      if (!s) return;
      isPlayingState = s.playing;

      document.getElementById('lbl-title').innerText = s.current_video ? s.current_video.split('/').pop().split('\\').pop() : 'Idle';
      
      const fmtTime = (sec) => {
        const m = Math.floor(sec / 60); const s = Math.floor(sec % 60);
        return `${m}:${s < 10 ? '0' : ''}${s}`;
      };
      document.getElementById('lbl-time').innerText = `${fmtTime(s.playback_time || 0)} / ${fmtTime(s.duration || 0)}`;

      if (s.duration > 0) {
        document.getElementById('seek-slider').value = ((s.playback_time / s.duration) * 100).toFixed(1);
      } else {
        document.getElementById('seek-slider').value = 0;
      }
      
      if (Number.isFinite(s.volume)) {
        const slider = document.getElementById('vol-slider');
        slider.disabled = false;
        slider.value = s.volume;
        document.getElementById('lbl-vol').innerText = `${Math.round(s.volume)}%`;
      }

      const playIcon = document.getElementById('icon-play');
      if (s.playing) {
        playIcon.innerHTML = '<path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/>';
      } else {
        playIcon.innerHTML = '<path d="M8 5v14l11-7z"/>';
      }

      if (s.current_video) {
        const now = Date.now();
        if (now - lastFrameUpdate > 1000) {
          refreshVideoFrame(s.current_video);
          lastFrameUpdate = now;
        }
      } else {
        document.getElementById('frame-box').innerHTML = `
          <div class="frame-placeholder">
            <svg viewBox="0 0 24 24" style="width: 48px; height: 48px; opacity: 0.5;"><path d="M21 3H3c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h18c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H3V5h18v14zM8 15c.55 0 1-.45 1-1s-.45-1-1-1-1 .45-1 1 .45 1 1 1zm4 0c.55 0 1-.45 1-1s-.45-1-1-1-1 .45-1 1 .45 1 1 1zm4 0c.55 0 1-.45 1-1s-.45-1-1-1-1 .45-1 1 .45 1 1 1z"/></svg>
            <div>No Video Loaded</div>
            <button class="nav-btn" style="margin-top: 0.5rem;" onclick="switchTab('explorer')">Open Media Explorer</button>
          </div>
        `;
      }
    }

    function refreshVideoFrame(videoPath = '') {
      const frameBox = document.getElementById('frame-box');
      const timestamp = Date.now();
      const frameUrl = videoPath ? `/api/player/frame?path=${encodeURIComponent(videoPath)}&t=${timestamp}` : `/api/player/frame?t=${timestamp}`;
      frameBox.innerHTML = `<img class="video-frame-img" src="${frameUrl}" onerror="this.style.display='none'">`;
    }

    function onSeek(val) {
      sendCmd('seek_abs', { percentage: parseFloat(val) });
    }

    function onVol(val) {
      document.getElementById('lbl-vol').innerText = `${val}%`;
      sendCmd('set_volume', { value: parseFloat(val) });
    }

    window.onload = () => {
      initWS();
    };
  </script>
</body>
</html>
"#;
