import { defineConfig } from 'vite'

// Plane ist eine Single-Page-Anwendung: alle Bildschirme liegen in
// frontend/index.html und werden per CSS-Klasse umgeschaltet. Es gibt daher
// genau einen Einstiegspunkt (frueher drei, mit dupliziertem Markup).
export default defineConfig({
  root: 'frontend',
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  build: {
    outDir: '../dist',
    emptyOutDir: true,
    // Tauri liefert eine feste Webview-Version aus; Transpilierung fuer alte
    // Browser waere reiner Ballast.
    target: 'esnext',
    minify: 'esbuild',
  },
})
