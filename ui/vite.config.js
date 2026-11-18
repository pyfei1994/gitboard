import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// base: './' 让产物既能被 Tauri 内嵌，也能直接 file:// 打开做截图检查
export default defineConfig({
  plugins: [vue()],
  base: './',
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    target: 'safari15',
  },
})
