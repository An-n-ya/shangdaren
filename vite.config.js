import {defineConfig} from 'vite'
export default defineConfig({
    server: {
        proxy: {
            '/api': 'ws://localhost:3131'
        }
    }
})