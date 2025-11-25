import path from "path";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { tanstackRouter } from "@tanstack/router-plugin/vite";

const isWatch = process.argv.includes("--watch");
// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    tanstackRouter({
      target: "react",
      autoCodeSplitting: true,
    }),
    react(),
    tailwindcss(),
  ],
  mode: isWatch ? "dev" : "production",
  build: {
    outDir: "../tierkreis_visualization/static/dist",
    emptyOutDir: true,
    minify: !isWatch,
    sourcemap: isWatch,
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
});
