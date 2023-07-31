import { defineConfig } from "vite";
import react from "@vitejs/plugin-react-swc";

const FRONTEND_BASE = process.env.FRONTEND_BASE_ENV || "/frontend/";
const DEFAULT_BACKEND_URL = process.env.DEFAULT_BACKEND_URL_ENV || "/api";

// https://vitejs.dev/config/
export default defineConfig({
    plugins: [react()],
    define: {
        APP_VERSION: JSON.stringify(process.env.npm_package_version),
        FRONTEND_BASE: JSON.stringify(FRONTEND_BASE),
        DEFAULT_BACKEND_URL: JSON.stringify(DEFAULT_BACKEND_URL),
    },
    base: FRONTEND_BASE,
    build: {
        outDir: "frontend",
        chunkSizeWarningLimit: 1500,
        rollupOptions: {
            output: {
                manualChunks(id) {
                    if (id.includes("node_modules")) {
                        return id.toString().split("node_modules/")[1].split("/")[0].toString();
                    }
                },
            },
        },
    },
});
