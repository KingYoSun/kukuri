/// prefers-reduced-motion の JS 判定を 1 箇所へ寄せる共有 helper。
///
/// OS 設定(matchMedia)を最優先に評価し、それに加えて review 環境
/// (Storybook decorator)が documentElement へ設定する
/// `data-reduced-motion='reduce'` も抑制対象に含める(OR)。
/// production はこの data 属性を設定しないため、実アプリの挙動は
/// これまでどおり matchMedia のみで決まる。
export function prefersReducedMotion(): boolean {
  if (typeof window === 'undefined' || typeof document === 'undefined') return false;
  const osPrefersReduce =
    window.matchMedia?.('(prefers-reduced-motion: reduce)').matches ?? false;
  return osPrefersReduce || document.documentElement.dataset.reducedMotion === 'reduce';
}
