pub const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Pealayer Web Remote</title>
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
    
    header { background: var(--glass); backdrop-filter: blur(12px); border-bottom: 1px solid var(--border); position: sticky; top: 0; z-index: 50; padding: 1rem 1.5rem; display: flex; justify-content: space-between; align-items: center; }
    .brand { font-size: 1.25rem; font-weight: 700; display: flex; align-items: center; gap: 0.5rem; }
    .status-badge { font-size: 0.75rem; padding: 0.25rem 0.6rem; border-radius: 9999px; background: rgba(34, 197, 94, 0.2); color: #4ADE80; display: inline-flex; align-items: center; gap: 0.35rem; font-weight: 600; }
    .status-badge.offline { background: rgba(239, 68, 68, 0.2); color: #F87171; }
    
    main { flex: 1; padding: 1.5rem; max-width: 650px; margin: 0 auto; width: 100%; display: flex; align-items: center; justify-content: center; }
    
    .remote-card { background: var(--bg-card); border: 1px solid var(--border); border-radius: 1.25rem; padding: 1.75rem; backdrop-filter: blur(12px); width: 100%; box-shadow: 0 20px 40px rgba(0,0,0,0.5); }
    .now-playing { text-align: center; }
    
    .video-frame-wrap { width: 100%; aspect-ratio: 16/9; background: #080810; border-radius: 0.85rem; border: 1px solid var(--border); display: flex; align-items: center; justify-content: center; overflow: hidden; margin-bottom: 1.25rem; position: relative; }
    .video-frame-img { width: 100%; height: 100%; object-fit: cover; transition: opacity 0.15s ease-in-out; }
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

    svg { width: 22px; height: 22px; fill: currentColor; }
  </style>
</head>
<body>
  <header>
    <div class="brand">
      <svg viewBox="0 0 24 24"><path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-2 14.5v-9l6 4.5-6 4.5z"/></svg>
      Pealayer Remote
    </div>
    <div class="status-badge" id="net-status">
      <span>●</span> Connecting...
    </div>
  </header>

  <main>
    <div class="remote-card">
      <div class="now-playing">
        <div class="video-frame-wrap" id="frame-box">
          <img id="video-frame-img" class="video-frame-img" style="display:none;" alt="Video Frame">
          <div class="frame-placeholder" id="frame-placeholder">
            <svg viewBox="0 0 24 24" style="width: 48px; height: 48px; opacity: 0.5;"><path d="M21 3H3c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h18c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H3V5h18v14zM8 15c.55 0 1-.45 1-1s-.45-1-1-1-1 .45-1 1 .45 1 1 1zm4 0c.55 0 1-.45 1-1s-.45-1-1-1-1 .45-1 1 .45 1 1 1zm4 0c.55 0 1-.45 1-1s-.45-1-1-1-1 .45-1 1 .45 1 1 1z"/></svg>
            No Video Loaded
          </div>
        </div>
        <div class="now-title" id="lbl-title">Idle</div>
        <div class="now-time" id="lbl-time">00:00 / 00:00</div>
      </div>

      <div class="seek-container">
        <input type="range" class="seek-bar" id="seek-slider" min="0" max="100" value="0" onchange="onSeek(this.value)">
      </div>

      <div class="control-row">
        <button class="ctrl-btn" title="Seek -10s" onclick="sendCmd('seek', {seconds: -10})">
          <svg viewBox="0 0 24 24"><path d="M11 18V6l-8.5 6 8.5 6zm.5-6l8.5 6V6l-8.5 6z"/></svg>
        </button>
        <button class="ctrl-btn play-btn" id="btn-play" title="Play / Pause" onclick="togglePlayPause()">
          <svg id="icon-play" viewBox="0 0 24 24" style="width: 28px; height: 28px;"><path d="M8 5v14l11-7z"/></svg>
        </button>
        <button class="ctrl-btn" title="Seek +10s" onclick="sendCmd('seek', {seconds: 10})">
          <svg viewBox="0 0 24 24"><path d="M4 18l8.5-6L4 6v12zm9-12v12l8.5-6L13 6z"/></svg>
        </button>
      </div>

      <div class="vol-container">
        <svg viewBox="0 0 24 24"><path d="M3 9v6h4l5 5V4L7 9H3zm13.5 3c0-1.77-1.02-3.29-2.5-4.03v8.05c1.48-.73 2.5-2.25 2.5-4.02z"/></svg>
        <input type="range" class="vol-bar" id="vol-slider" min="0" max="130" value="100" oninput="onVol(this.value)">
        <span id="lbl-vol" style="font-size: 0.85rem; width: 45px; text-align: right; font-family: monospace;">100%</span>
      </div>
    </div>
  </main>

  <script>
    let ws;
    let isPlayingState = false;
    let currentVideoPath = '';
    let lastFrameUpdate = 0;

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

      // Smoothly refresh video frame snapshot during active playback
      setInterval(() => {
        if (isPlayingState && currentVideoPath) {
          const now = Date.now();
          if (now - lastFrameUpdate > 1000) {
            refreshVideoFrame(currentVideoPath);
            lastFrameUpdate = now;
          }
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

    function togglePlayPause() {
      // Optimistic UI toggle for instant feedback
      isPlayingState = !isPlayingState;
      renderPlayIcon(isPlayingState);
      sendCmd('toggle_pause');
    }

    function renderPlayIcon(playing) {
      const playIcon = document.getElementById('icon-play');
      if (playing) {
        playIcon.innerHTML = '<path d="M6 19h4V5H6v14zm8-14v14h4V5h-4z"/>';
      } else {
        playIcon.innerHTML = '<path d="M8 5v14l11-7z"/>';
      }
    }

    function updateRemoteUI(s) {
      if (!s) return;
      isPlayingState = s.playing;
      currentVideoPath = s.current_video || '';

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
      
      document.getElementById('vol-slider').value = s.volume || 100;
      document.getElementById('lbl-vol').innerText = `${Math.round(s.volume || 100)}%`;

      renderPlayIcon(s.playing);

      if (s.current_video) {
        const now = Date.now();
        if (now - lastFrameUpdate > 1200) {
          refreshVideoFrame(s.current_video);
          lastFrameUpdate = now;
        }
      } else {
        document.getElementById('video-frame-img').style.display = 'none';
        document.getElementById('frame-placeholder').style.display = 'flex';
      }
    }

    function refreshVideoFrame(videoPath) {
      const frameImg = document.getElementById('video-frame-img');
      const placeholder = document.getElementById('frame-placeholder');
      const timestamp = Date.now();
      const frameUrl = `/api/player/frame?path=${encodeURIComponent(videoPath)}&t=${timestamp}`;
      
      // Preload offscreen to eliminate DOM flickering
      const preload = new Image();
      preload.onload = () => {
        frameImg.src = preload.src;
        frameImg.style.display = 'block';
        placeholder.style.display = 'none';
      };
      preload.src = frameUrl;
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
