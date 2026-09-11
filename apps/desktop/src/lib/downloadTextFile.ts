// 診断レポート(#958 系)とアプリ内ログ(#978)が共有する、利用者操作による text 書き出し。
// Blob + <a download> は WebKitGTK / WebView2 の両方で「保存先を利用者が選ぶ」既存経路。
// 自動保存・自動送信には使わない。
export function downloadTextFile(fileName: string, text: string): void {
  const blob = new Blob([text], { type: 'text/plain;charset=utf-8' });
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = fileName;
  link.click();
  URL.revokeObjectURL(url);
}
