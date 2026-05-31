# p1-t01: プロジェクトセットアップ — tsconfig厳格化 + TypeScript型基盤

仕様書参照: §17 TypeScript型安全基盤、§21 Phase 1（プロジェクトセットアップ = タスク群1、TypeScript型基盤 = タスク群1の細目）

## 前提

- Tauri + SolidJS + TypeScript のスキャフォールドは完了済み
- `src/` ディレクトリ、`tsconfig.json` が存在する

## 作業内容

### 1. tsconfig.json の更新

以下のオプションに変更する（Liminia Type Safety v1 準拠）:

```jsonc
{
  "compilerOptions": {
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "erasableSyntaxOnly": true,
    "verbatimModuleSyntax": true,
    "target": "ES2022",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "preserve",
    "jsxImportSource": "solid-js",
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "forceConsistentCasingInFileNames": true,
    "esModuleInterop": true
  },
  "include": ["src"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

注意点（C1/C3）:
- `erasableSyntaxOnly` は **TypeScript 5.8+** で有効。`package.json` の `typescript` が 5.8 未満なら先に上げる（`pnpm add -D typescript@latest`）。5.8 未満のままだと未知のオプションとして無視され、`enum` 禁止が効かない。
- `esModuleInterop: true` は仕様書 §17 に含まれるため追加する（上記ブロックに反映済み）。
- `references: [{ "path": "./tsconfig.node.json" }]` は `tsconfig.node.json` が存在する前提。リポジトリには存在するが、念のため存在を確認し、無ければ `references` 行を削除する。

### 2. src/types/ ディレクトリと型定義ファイルの作成

`src/types/brand.ts`:
```typescript
declare const __brand: unique symbol;
export type Brand<T, B extends string> = T & { readonly [__brand]: B };

export type RelativePath = Brand<string, "RelativePath">;
export type AbsolutePath = Brand<string, "AbsolutePath">;

export function toRelativePath(s: string): RelativePath {
  // TODO(Phase 2): 検証を実装する（絶対パス・`..` 親参照・先頭スラッシュの拒否等）。
  // 仕様書 §17 は検証付きコンストラクタを意図しているが、Phase 1 では未実装の素通しキャスト。
  return s as RelativePath;
}

export function toAbsolutePath(s: string): AbsolutePath {
  // TODO(Phase 2): 検証を実装する。Phase 1 では未実装の素通しキャスト。
  return s as AbsolutePath;
}
```

> C2: 上記コンストラクタは Phase 1 では検証なしの `as` キャストにとどめる（仕様書の `/* validation */` は Phase 2 で実装）。「検証済み」と誤認しないよう TODO を残す。

`src/types/constants.ts`:
```typescript
export const VIEW_MODE = {
  EDITOR: "editor",
  SPLIT: "split",
  PREVIEW: "preview",
} as const;
export type ViewMode = (typeof VIEW_MODE)[keyof typeof VIEW_MODE];

export const THEME = { DARK: "dark", LIGHT: "light" } as const;
export type Theme = (typeof THEME)[keyof typeof THEME];

export const SAVE_STATE = {
  SAVED: "saved",
  UNSAVED: "unsaved",
  SAVING: "saving",
} as const;
export type SaveState = (typeof SAVE_STATE)[keyof typeof SAVE_STATE];
```

`src/types/models.ts`: Tauriコマンド応答型を定義する（タスク05で順次追記するため、今は空の re-export ファイルとして作成）:
```typescript
// Tauri command response types — populated in p1-t05
export type {};
```

### 3. 型チェックの確認

```bash
pnpm exec tsc --noEmit
```

既存の `src/App.tsx` が `erasableSyntaxOnly` に違反している場合（enum使用等）は修正する。SolidJSの標準テンプレートはほぼ問題ないはず。

## 完了基準

- `pnpm exec tsc --noEmit` がエラーゼロで通る
- `src/types/brand.ts`, `src/types/constants.ts`, `src/types/models.ts` が存在する
- tsconfig に `noUncheckedIndexedAccess`, `erasableSyntaxOnly`, `verbatimModuleSyntax` が設定されている

## 注意

- `noUncheckedIndexedAccess` を有効にすると `arr[0]` の型が `T | undefined` になる。既存コードがあれば適宜修正する。
- `verbatimModuleSyntax` により `import type` の使い方が厳格化される。型のみのインポートには必ず `import type` を使う。
- `erasableSyntaxOnly` により `enum` と `namespace` は使用不可（`as const` オブジェクトで代替）。
