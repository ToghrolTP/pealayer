pub const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Pealayer Web Remote & Explorer</title>
  <style>
    :root {
      --bg-dark: #000000;
      --bg-card: #0F0F23;
      --bg-surface: #1E1B4B;
      --accent: #E11D48;
      --accent-hover: #F43F5E;
      --text-main: #F8FAFC;
      --text-muted: #94A3B8;
      --border: rgba(255, 255, 255, 0.1);
      --glass: rgba(15, 15, 35, 0.75);
    }
    * { box-sizing: border-box; margin: 0; padding: 0; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; }
    body { background: var(--bg-dark); color: var(--text-main); min-height: 100vh; display: flex; flex-direction: column; overflow-x: hidden; }
    
    header { background: var(--glass); backdrop-filter: blur(12px); border-bottom: 1px solid var(--border); position: sticky; top: 0; z-index: 50; padding: 1rem 1.5rem; display: flex; justify-content: space-between; align-items: center; }
    .brand { font-size: 1.25rem; font-weight: 700; display: flex; align-items: center; gap: 0.5rem; }
    .status-badge { font-size: 0.75rem; padding: 0.25rem 0.6rem; border-radius: 9999px; background: rgba(34, 197, 94, 0.2); color: #4ADE80; display: inline-flex; align-items: center; gap: 0.35rem; }
    .status-badge.offline { background: rgba(239, 68, 68, 0.2); color: #F87171; }
    
    nav { display: flex; gap: 0.5rem; }
    .nav-btn { background: transparent; border: 1px solid transparent; color: var(--text-muted); padding: 0.5rem 1rem; border-radius: 0.5rem; font-weight: 600; cursor: pointer; transition: all 0.2s; }
    .nav-btn:hover { color: var(--text-main); background: rgba(255,255,255,0.05); }
    .nav-btn.active { color: var(--text-main); background: var(--bg-surface); border-color: var(--border); }
    
    main { flex: 1; padding: 1.5rem; max-width: 1200px; margin: 0 auto; width: 100%; }
    .tab-content { display: none; }
    .tab-content.active { display: block; }
    
    /* Remote Tab */
    .remote-grid { display: grid; grid-template-columns: 1fr; gap: 1.5rem; max-width: 650px; margin: 0 auto; }
    .remote-card { background: var(--bg-card); border: 1px solid var(--border); border-radius: 1rem; padding: 1.5rem; backdrop-filter: blur(8px); }
    .now-playing { text-align: center; }
    .poster-placeholder { width: 100%; aspect-ratio: 16/9; background: #111; border-radius: 0.75rem; border: 1px solid var(--border); display: flex; align-items: center; justify-content: center; overflow: hidden; margin-bottom: 1rem; position: relative; }
    .poster-img { width: 100%; height: 100%; object-fit: cover; }
    .now-title { font-size: 1.1rem; font-weight: 600; margin-bottom: 0.25rem; word-break: break-all; }
    .now-time { font-size: 0.875rem; color: var(--text-muted); font-family: monospace; }
    
    .seek-container { margin: 1.25rem 0; display: flex; align-items: center; gap: 0.75rem; }
    .seek-bar { flex: 1; accent-color: var(--accent); height: 6px; cursor: pointer; }
    
    .control-row { display: flex; justify-content: center; align-items: center; gap: 1.25rem; margin-top: 1rem; }
    .ctrl-btn { background: var(--bg-surface); border: 1px solid var(--border); color: var(--text-main); width: 44px; height: 44px; border-radius: 50%; display: flex; align-items: center; justify-content: center; cursor: pointer; transition: all 0.2s; }
    .ctrl-btn:hover { background: rgba(255,255,255,0.15); transform: scale(1.05); }
    .ctrl-btn.play-btn { width: 56px; height: 56px; background: var(--accent); border: none; }
    .ctrl-btn.play-btn:hover { background: var(--accent-hover); }

    .vol-container { display: flex; align-items: center; gap: 0.75rem; margin-top: 1.25rem; background: rgba(255,255,255,0.03); padding: 0.75rem 1rem; border-radius: 0.75rem; }
    .vol-bar { flex: 1; accent-color: var(--accent); height: 5px; cursor: pointer; }

    /* Explorer Tab */
    .explorer-toolbar { display: flex; flex-wrap: wrap; gap: 0.75rem; justify-content: space-between; align-items: center; margin-bottom: 1.25rem; }
    .breadcrumbs { display: flex; align-items: center; gap: 0.35rem; font-size: 0.875rem; color: var(--text-muted); flex-wrap: wrap; }
    .crumb { cursor: pointer; color: var(--accent); font-weight: 500; }
    .crumb:hover { text-decoration: underline; }
    
    .search-input { background: var(--bg-card); border: 1px solid var(--border); color: var(--text-main); padding: 0.5rem 0.85rem; border-radius: 0.5rem; font-size: 0.875rem; min-width: 220px; }
    
    .media-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 1rem; }
    .media-card { background: var(--bg-card); border: 1px solid var(--border); border-radius: 0.75rem; overflow: hidden; display: flex; flex-direction: column; transition: all 0.2s; position: relative; }
    .media-card:hover { border-color: var(--accent); transform: translateY(-2px); }
    .thumb-wrap { width: 100%; aspect-ratio: 16/9; background: #080812; display: flex; align-items: center; justify-content: center; cursor: pointer; overflow: hidden; }
    .thumb-img { width: 100%; height: 100%; object-fit: cover; }
    .media-info { padding: 0.75rem; display: flex; justify-content: space-between; align-items: flex-start; gap: 0.5rem; }
    .media-name { font-size: 0.85rem; font-weight: 500; word-break: break-all; cursor: pointer; flex: 1; }
    .card-actions { display: flex; gap: 0.25rem; }
    .act-btn { background: transparent; border: none; color: var(--text-muted); padding: 0.25rem; border-radius: 0.25rem; cursor: pointer; }
    .act-btn:hover { color: var(--text-main); background: rgba(255,255,255,0.1); }
    
    /* Modal */
    .modal-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.7); backdrop-filter: blur(4px); display: none; align-items: center; justify-content: center; z-index: 100; }
    .modal-overlay.active { display: flex; }
    .modal-box { background: var(--bg-card); border: 1px solid var(--border); border-radius: 1rem; width: 90%; max-width: 400px; padding: 1.5rem; }
    .modal-title { font-size: 1.1rem; font-weight: 700; margin-bottom: 1rem; }
    .modal-input { width: 100%; background: var(--bg-dark); border: 1px solid var(--border); color: var(--text-main); padding: 0.6rem 0.75rem; border-radius: 0.5rem; margin-bottom: 1rem; }
    .modal-btns { display: flex; justify-content: flex-end; gap: 0.5rem; }
    .btn { padding: 0.5rem 1rem; border-radius: 0.5rem; font-weight: 600; cursor: pointer; border: none; }
    .btn-secondary { background: var(--bg-surface); color: var(--text-main); }
    .btn-primary { background: var(--accent); color: var(--text-main); }
    .btn-danger { background: #DC2626; color: var(--text-main); }
    
    svg { width: 20px; height: 20px; fill: currentColor; }
  </style>
</head>
<body>
  <header>
    <div class="brand">
      <svg viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 14.5v-9l6 4.5-6 4.5z"/></svg>
      Pealayer Remote
    </div>
    <div class="status-badge" id="net-status">
      <span>●</span> Connected
    </div>
    <nav>
      <button class="nav-btn active" onclick="switchTab('remote')">Remote</button>
      <button class="nav-btn" onclick="switchTab('explorer')">Explorer</button>
    </nav>
  </header>

  <main>
    <!-- Remote Control Tab -->
    <div id="tab-remote" class="tab-content active">
      <div class="remote-grid">
        <div class="remote-card">
          <div class="now-playing">
            <div class="poster-placeholder" id="poster-box">
              <span style="color: var(--text-muted);">No Media Playing</span>
            </div>
            <div class="now-title" id="lbl-title">Idle</div>
            <div class="now-time" id="lbl-time">00:00 / 00:00</div>
          </div>

          <div class="seek-container">
            <input type="range" class="seek-bar" id="seek-slider" min="0" max="100" value="0" onchange="onSeek(this.value)">
          </div>

          <div class="control-row">
            <button class="ctrl-btn" onclick="sendCmd('seek', {seconds: -10})">
              <svg viewBox="0 0 24 24"><path d="M11 18V6l-8.5 6 8.5 6zm.5-6l8.5 6V6l-8.5 6z"/></svg>
            </button>
            <button class="ctrl-btn play-btn" id="btn-play" onclick="sendCmd('toggle_pause')">
              <svg id="icon-play" viewBox="0 0 24 24"><path d="M8 5v14l11-7z"/></svg>
            </button>
            <button class="ctrl-btn" onclick="sendCmd('seek', {seconds: 10})">
              <svg viewBox="0 0 24 24"><path d="M4 18l8.5-6L4 6v12zm9-12v12l8.5-6L13 6z"/></svg>
            </button>
          </div>

          <div class="vol-container">
            <svg viewBox="0 0 24 24"><path d="M3 9v6h4l5 5V4L7 9H3zm13.5 3c0-1.77-1.02-3.29-2.5-4.03v8.05c1.48-.73 2.5-2.25 2.5-4.02z"/></svg>
            <input type="range" class="vol-bar" id="vol-slider" min="0" max="130" value="100" oninput="onVol(this.value)">
            <span id="lbl-vol" style="font-size: 0.85rem; width: 40px; text-align: right;">100%</span>
          </div>
        </div>
      </div>
    </div>

    <!-- Media Explorer Tab -->
    <div id="tab-explorer" class="tab-content">
      <div class="explorer-toolbar">
        <div class="breadcrumbs" id="crumb-box">/</div>
        <input type="text" class="search-input" id="search-box" placeholder="Search media..." oninput="filterMedia()">
      </div>

      <div class="media-grid" id="media-grid">
        <!-- Rendered dynamically -->
      </div>
    </div>
  </main>

  <!-- Modal Rename -->
  <div class="modal-overlay" id="modal-rename">
    <div class="modal-box">
      <div class="modal-title">Rename File</div>
      <input type="text" class="modal-input" id="txt-rename">
      <div class="modal-btns">
        <button class="btn btn-secondary" onclick="closeModal('modal-rename')">Cancel</button>
        <button class="btn btn-primary" onclick="submitRename()">Rename</button>
      </div>
    </div>
  </div>

  <!-- Modal Trash -->
  <div class="modal-overlay" id="modal-trash">
    <div class="modal-box">
      <div class="modal-title">Move to Trash?</div>
      <p style="color: var(--text-muted); font-size: 0.9rem; margin-bottom: 1rem;" id="txt-trash-desc">Are you sure you want to delete this file?</p>
      <div class="modal-btns">
        <button class="btn btn-secondary" onclick="closeModal('modal-trash')">Cancel</button>
        <button class="btn btn-danger" onclick="submitTrash()">Delete</button>
      </div>
    </div>
  </div>

  <script>
    let ws;
    let currentDirData = null;
    let activeActionPath = '';

    function initWS() {
      const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
      const wsUrl = `${proto}//${location.hostname}:8081`;
      
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

      // REST Polling Fallback (ensures video detection even if WebSockets are blocked)
      setInterval(async () => {
        try {
          const res = await fetch('/api/player/status');
          if (res.ok) {
            const state = await res.json();
            updateRemoteUI(state);
            if (!ws || ws.readyState !== WebSocket.OPEN) {
              document.getElementById('net-status').classList.remove('offline');
              document.getElementById('net-status').innerHTML = '<span>●</span> Online (REST)';
            }
          }
        } catch(e) {}
      }, 500);
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

    function updateRemoteUI(s) {
      if (!s) return;
      document.getElementById('lbl-title').innerText = s.current_video ? s.current_video.split('/').pop() : 'Idle';
      
      const fmtTime = (sec) => {
        const m = Math.floor(sec / 60); const s = Math.floor(sec % 60);
        return `${m}:${s < 10 ? '0' : ''}${s}`;
      };
      document.getElementById('lbl-time').innerText = `${fmtTime(s.playback_time || 0)} / ${fmtTime(s.duration || 0)}`;

      if (s.duration > 0) {
        document.getElementById('seek-slider').value = ((s.playback_time / s.duration) * 100).toFixed(1);
      }
      
      document.getElementById('vol-slider').value = s.volume || 100;
      document.getElementById('lbl-vol').innerText = `${Math.round(s.volume || 100)}%`;

      const playIcon = document.getElementById('icon-play');
      if (s.playing) {
        playIcon.innerHTML = '<path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/>';
      } else {
        playIcon.innerHTML = '<path d="M8 5v14l11-7z"/>';
      }

      if (s.current_video) {
        document.getElementById('poster-box').innerHTML = `<img class="poster-img" src="/api/fs/thumbnail?path=${encodeURIComponent(s.current_video)}">`;
      }
    }

    function onSeek(val) {
      sendCmd('seek_abs', { percentage: parseFloat(val) });
    }

    function onVol(val) {
      document.getElementById('lbl-vol').innerText = `${val}%`;
      sendCmd('set_volume', { value: parseFloat(val) });
    }

    function switchTab(tab) {
      document.querySelectorAll('.nav-btn').forEach(b => b.classList.remove('active'));
      document.querySelectorAll('.tab-content').forEach(t => t.classList.remove('active'));
      event.target.classList.add('active');
      document.getElementById(`tab-${tab}`).classList.add('active');

      if (tab === 'explorer' && !currentDirData) {
        loadDirectory('');
      }
    }

    async function loadDirectory(path = '') {
      const res = await fetch(`/api/fs/browse?path=${encodeURIComponent(path)}`);
      if (res.ok) {
        currentDirData = await res.json();
        renderExplorer();
      }
    }

    function renderExplorer() {
      if (!currentDirData) return;
      
      // Crumb box
      const crumbs = currentDirData.current_path.split('/');
      let pathAcc = '';
      let crumbHTML = '';
      crumbs.forEach((c, idx) => {
        if (c === '' && idx === 0) return;
        pathAcc += '/' + c;
        const targetP = pathAcc;
        crumbHTML += `<span class="crumb" onclick="loadDirectory('${encodeURIComponent(targetP)}')">${c || '/'}</span> / `;
      });
      document.getElementById('crumb-box').innerHTML = crumbHTML || '/';

      const query = document.getElementById('search-box').value.toLowerCase();
      const grid = document.getElementById('media-grid');
      grid.innerHTML = '';

      currentDirData.entries.forEach(e => {
        if (query && !e.name.toLowerCase().includes(query)) return;

        const card = document.createElement('div');
        card.className = 'media-card';

        if (e.is_dir) {
          card.innerHTML = `
            <div class="thumb-wrap" onclick="loadDirectory('${encodeURIComponent(e.path)}')">
              <svg viewBox="0 0 24 24" style="width: 48px; height: 48px; color: var(--accent);"><path d="M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z"/></svg>
            </div>
            <div class="media-info">
              <span class="media-name" onclick="loadDirectory('${encodeURIComponent(e.path)}')">${e.name}</span>
            </div>
          `;
        } else {
          const thumbUrl = e.has_thumbnail ? `/api/fs/thumbnail?path=${encodeURIComponent(e.path)}` : '';
          card.innerHTML = `
            <div class="thumb-wrap" onclick="playMedia('${encodeURIComponent(e.path)}')">
              ${thumbUrl ? `<img class="thumb-img" src="${thumbUrl}">` : `<svg viewBox="0 0 24 24" style="width: 40px; height: 40px; color: var(--text-muted);"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 14.5v-9l6 4.5-6 4.5z"/></svg>`}
            </div>
            <div class="media-info">
              <span class="media-name" onclick="playMedia('${encodeURIComponent(e.path)}')">${e.name}</span>
              <div class="card-actions">
                <button class="act-btn" title="Rename" onclick="openRenameModal('${encodeURIComponent(e.path)}', '${e.name.replace(/'/g, "\\'")}')">
                  <svg viewBox="0 0 24 24"><path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34c-.39-.39-1.02-.39-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z"/></svg>
                </button>
                <button class="act-btn" title="Trash" onclick="openTrashModal('${encodeURIComponent(e.path)}', '${e.name.replace(/'/g, "\\'")}')">
                  <svg viewBox="0 0 24 24"><path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/></svg>
                </button>
              </div>
            </div>
          `;
        }
        grid.appendChild(card);
      });
    }

    function filterMedia() { renderExplorer(); }

    function playMedia(path) {
      sendCmd('open', { target: decodeURIComponent(path) });
      switchTab('remote');
    }

    function openRenameModal(path, name) {
      activeActionPath = decodeURIComponent(path);
      document.getElementById('txt-rename').value = name;
      document.getElementById('modal-rename').classList.add('active');
    }

    function openTrashModal(path, name) {
      activeActionPath = decodeURIComponent(path);
      document.getElementById('txt-trash-desc').innerText = `Are you sure you want to delete "${name}"?`;
      document.getElementById('modal-trash').classList.add('active');
    }

    function closeModal(id) {
      document.getElementById(id).classList.remove('active');
    }

    async function submitRename() {
      const newName = document.getElementById('txt-rename').value.trim();
      if (newName && activeActionPath) {
        const res = await fetch('/api/fs/rename', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ old_path: activeActionPath, new_name: newName })
        });
        if (res.ok) {
          closeModal('modal-rename');
          loadDirectory(currentDirData ? currentDirData.current_path : '');
        }
      }
    }

    async function submitTrash() {
      if (activeActionPath) {
        const res = await fetch('/api/fs/trash', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ target_path: activeActionPath })
        });
        if (res.ok) {
          closeModal('modal-trash');
          loadDirectory(currentDirData ? currentDirData.current_path : '');
        }
      }
    }

    window.onload = () => {
      initWS();
    };
  </script>
</body>
</html>
"#;
