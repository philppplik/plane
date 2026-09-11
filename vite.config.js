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
    // Tauri liefert eine feste Webview-Version aus; Transpilierung für alte
    // Browser wäre reiner Ballast.
    target: 'esnext',
    // Bewusst `true` statt eines festen Minifizierers: Vite hat den
    // Unterbau von esbuild auf rolldown umgestellt. `minify: 'esbuild'`
    // würde dort ein Paket verlangen, das nicht mehr mitgeliefert wird.
    minify: true,
  },
})
