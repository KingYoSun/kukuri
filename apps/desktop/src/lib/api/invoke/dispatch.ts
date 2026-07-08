import type { DesktopApi } from '../types';

/// mock ビルド(dev / Storybook / Playwright / テスト)では window.__KUKURI_DESKTOP__ に
/// in-memory 実装が後から注入される。ディスパッチは**呼び出し時点**でその存在を見る
/// (モジュール初期化時に固定してはならない)。WP-H7 PR1 で 82 メソッド + 3 スタンドアロン
/// 関数に散っていた `if (window.__KUKURI_DESKTOP__)` 分岐をここへ集約した。

export function isDesktopMockActive(): boolean {
  return Boolean(window.__KUKURI_DESKTOP__);
}

/// DesktopApi のメソッド 1 本を mock 対応でラップする。mock が居れば同名メソッドへ、
/// 居なければ invoke 実装(引数 → snake_case request 構築)へ、同じ引数のまま転送する。
/// 引数・戻り値の型は DesktopApi[K] から保存されるため、転送のミスは tsc が検出する。
export function command<K extends keyof DesktopApi>(name: K, impl: DesktopApi[K]): DesktopApi[K] {
  const dispatch = (...args: unknown[]) => {
    const mock = window.__KUKURI_DESKTOP__;
    if (mock) {
      // mock オブジェクトのメソッドとして呼ぶ(this を束縛する)。
      // 一部の mock 実装は内部で this.otherMethod() を呼ぶため、
      // 変数へ取り出して素の関数として呼ぶと this が外れて壊れる。
      return (mock[name] as (...forwarded: unknown[]) => unknown).apply(mock, args);
    }
    return (impl as (...forwarded: unknown[]) => unknown)(...args);
  };
  return dispatch as DesktopApi[K];
}
