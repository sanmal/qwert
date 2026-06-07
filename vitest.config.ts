import solid from "vite-plugin-solid";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [solid()],
  test: {
    environment: "jsdom",
    globals: true,                       // describe/it/expect を import 不要に
    setupFiles: ["./vitest.setup.ts"],
    include: ["src/**/*.{test,spec}.{ts,tsx}"],
  },
  // 既知バグ（"dispose is undefined" 等、solid が二重ロードされる症状）が
  // 出たときのフォールバックとしてのみ有効化する。2.11 では通常不要。
  // resolve: { conditions: ["development", "browser"] },
});
