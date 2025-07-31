/**
 * 投稿の下書き
 */
export interface PostDraft {
  id: string;
  content: string;
  topicId: string | null;
  topicName?: string;
  scheduledDate: Date | null;
  createdAt: Date;
  updatedAt: Date;
  metadata?: {
    replyTo?: string;
    quotedPost?: string;
    attachments?: string[];
  };
}

/**
 * 下書きの作成パラメータ
 */
export interface CreateDraftParams {
  content: string;
  topicId: string | null;
  topicName?: string;
  scheduledDate?: Date | null;
  metadata?: PostDraft['metadata'];
}

/**
 * 下書きの更新パラメータ
 */
export interface UpdateDraftParams extends Partial<CreateDraftParams> {
  id: string;
}