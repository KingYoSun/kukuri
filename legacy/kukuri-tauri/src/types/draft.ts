import type { PostScope } from '@/stores/types';

/**
 * 投稿の下書き
 */
export interface PostDraft {
  id: string;
  content: string;
  topicId: string | null;
  topicName?: string;
  createdAt: Date;
  updatedAt: Date;
  metadata?: {
    replyTo?: string;
    quotedPost?: string;
    attachments?: string[];
    scope?: PostScope;
  };
}

/**
 * 下書きの作成パラメータ
 */
export interface CreateDraftParams {
  content: string;
  topicId: string | null;
  topicName?: string;
  metadata?: PostDraft['metadata'];
}

/**
 * 下書きの更新パラメータ
 */
export interface UpdateDraftParams extends Partial<CreateDraftParams> {
  id: string;
}
