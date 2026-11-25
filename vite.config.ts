import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// Konfigurasi untuk Tauri: port tetap, dan folder Rust tidak dipantau.
export default defineConfig({
  plugins: [vue()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: { ignored: ['**/src-tauri/**'] },
  },
  build: {
    // WebView2 di Windows 10/11 berbasis Chromium modern.
    target: 'chrome105',
    // Tanpa source map di rilis supaya exe kecil.
    sourcemap: false,
  },
})
