import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [
    react(),
    {
      name: "electron-file-protocol",
      transformIndexHtml(html) {
        return html.replace(/ crossorigin/g, "");
      },
    },
  ],
  server: { port: 5173, strictPort: true },
  base: "./",
});
