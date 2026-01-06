import path from "path";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import { tanstackRouter } from "@tanstack/router-plugin/vite";
import fs from "node:fs";
import openapiTS, { astToString } from "openapi-typescript";
import { Plugin } from "vite";

const openapiSpecFile = `${__dirname}/../openapi.json`;
const tsStubsFile = `${__dirname}/src/data/api_stubs.d.ts`;

function openapi(): Plugin {
  return {
    name: "openapi_vite_plugin",
    async transform() {
      const ast = await openapiTS(new URL(openapiSpecFile, import.meta.url));
      const contents = astToString(ast);
      fs.writeFileSync(tsStubsFile, contents);
    },
  };
}

const isWatch = process.argv.includes("--watch");
// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    openapi(),
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
