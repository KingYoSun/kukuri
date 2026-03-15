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
