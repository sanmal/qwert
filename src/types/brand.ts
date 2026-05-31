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
