import tailwindcss from "@tailwindcss/vite"
import react from "@vitejs/plugin-react"
import { defineConfig } from "vite"

export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": new URL("./src", import.meta.url).pathname,
    },
  },
  server: {
    proxy: {
      // Перевизначення — щоб ганяти dev-фронт проти віддаленого сервера (SSH-тунель на прод тощо).
      "/api": process.env.API_PROXY_TARGET ?? "http://localhost:3000",
    },
  },
})
