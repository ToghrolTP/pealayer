import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  base: './',
  // Keep responsive development wired to the real local Rust backend.
  server: {
    host: '127.0.0.1',
    proxy: {
      '/api': 'http://127.0.0.1:8080',
      '/ws': { target: 'ws://127.0.0.1:8080', ws: true },
    },
  },
  build: {
    outDir: 'dist',
    assetsDir: 'assets',
    emptyOutDir: true,
    // All surfaces share the application stylesheet. Version-stamped lazy CSS
    // URLs are not recognized by Vite's suffix-based preload helper, so do not
    // split CSS into dynamic chunks that can silently load as JavaScript.
    cssCodeSplit: false,
    // Readable filenames; finalize-pwa stamps references with the build
    // revision so browser/offline caches still distinguish deployments.
    rollupOptions: {
      output: {
        entryFileNames: 'assets/app.js',
        chunkFileNames: 'assets/[name].js',
        assetFileNames: (assetInfo) => {
          const names = (assetInfo as any).names ?? ((assetInfo as any).name ? [(assetInfo as any).name] : []);
          return names.some((name: string) => name === 'index.css' || name === 'style.css')
            ? 'assets/app.css' : 'assets/[name][extname]';
        },
      },
    },
    rolldownOptions: {
      output: {
        entryFileNames: 'assets/app.js',
        chunkFileNames: 'assets/[name].js',
        assetFileNames: (assetInfo: any) => {
          const names = assetInfo.names ?? (assetInfo.name ? [assetInfo.name] : []);
          return names.some((name: string) => name === 'index.css' || name === 'style.css')
            ? 'assets/app.css' : 'assets/[name][extname]';
        },
      },
    } as any,
  },
});
