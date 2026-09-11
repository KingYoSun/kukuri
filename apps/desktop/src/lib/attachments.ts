import type { CreateAttachmentInput } from './api';

function readBlobAsDataUrl(blob: Blob): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => {
      reject(reader.error ?? new Error('failed to read attachment data'));
    };
    reader.onload = () => {
      if (typeof reader.result !== 'string') {
        reject(new Error('failed to encode attachment data'));
        return;
      }
      resolve(reader.result);
    };
    reader.readAsDataURL(blob);
  });
}

export async function blobToBase64(blob: Blob): Promise<string> {
  const dataUrl = await readBlobAsDataUrl(blob);
  const marker = dataUrl.indexOf(',');
  if (marker < 0) {
    throw new Error('failed to encode attachment data');
  }
  return dataUrl.slice(marker + 1);
}

export async function blobToCreateAttachment(
  blob: Blob,
  fileName: string,
  role: CreateAttachmentInput['role']
): Promise<CreateAttachmentInput> {
  return {
    file_name: fileName,
    mime: blob.type || 'application/octet-stream',
    byte_size: blob.size,
    data_base64: await blobToBase64(blob),
    role,
  };
}

export async function fileToCreateAttachment(
  file: File,
  role: CreateAttachmentInput['role']
): Promise<CreateAttachmentInput> {
  return blobToCreateAttachment(file, file.name, role);
}

type TranslateAttachmentMessage = (key: string, options?: Record<string, unknown>) => string;

// #965: 非対応ファイルの理由は投稿・返信・DM で同じ文言にする。先頭のファイル名と残り件数だけを
// 出し、対応形式(画像と動画)の説明は locale 側の文言が持つ。判定自体は caller が所有する。
export function formatUnsupportedAttachmentMessage(
  translate: TranslateAttachmentMessage,
  rejectedNames: string[]
): string | null {
  const [name, ...others] = rejectedNames;
  if (name === undefined) {
    return null;
  }
  return others.length === 0
    ? translate('common:errors.unsupportedAttachmentType', { name })
    : translate('common:errors.unsupportedAttachmentTypes', { name, others: others.length });
}
