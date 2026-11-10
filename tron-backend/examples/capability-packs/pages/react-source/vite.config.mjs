import { defineConfig } from "vite";
export default defineConfig({ define: { "process.env.NODE_ENV": JSON.stringify("production") }, build: { outDir: "../ui", emptyOutDir: false, lib: { entry: "main.tsx", formats: ["iife"], name: "TronExample", fileName: () => "react-page.js" }, rollupOptions: { output: { inlineDynamicImports: true } } } });
