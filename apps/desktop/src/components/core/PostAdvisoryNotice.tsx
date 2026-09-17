import type { TFunction } from 'i18next';
import { useRef } from 'react';
import { useTranslation } from 'react-i18next';

import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';

import {
  advisoryBasisLabel,
  advisoryCategoryLabel,
  advisoryIssuerLabel,
  shortenNodeId,
} from './contentAdvisoryPresentation';
import type { ContentAdvisoryView } from './types';
import type { usePostAdvisoryDetails } from './usePostAdvisoryDetails';

/// #1055: Community Node の content advisory による代替表示の説明。
///
/// 判定は issuer node の node-local な推定であり、投稿者の申告でも kukuri ネットワーク全体の
/// 判断でもない(ADR 0046 §6.3 / ADR 0027 §2.8)。断定表現を避け、発行元・分類・確信度・根拠を
/// 必ず一緒に示す。#1108 以降は一覧に常時展開せず、詳細 dialog の中で表示する。
export type PostAdvisoryNoticeProps = {
  advisory: ContentAdvisoryView;
  objectId: string;
};

/// #858 / #1055: 表示設定 OFF の代替表示。本文の文言は判定元(投稿者の自己申告 / ノードの推定)で
/// 分け、canonical 解決前は呼出元が渡した待機文言を保つ。
/// #1108: advisory 付きの投稿は説明を一覧に出さず、詳細を開く操作だけを置く。メディア枠が
/// 詳細を開く操作を持つ場合(`onOpenDetails` 未指定)は、本文欄に何も出さない。
export type PostGatedContentProps = {
  objectId: string;
  gatedBy?: 'self_label' | 'advisory';
  bodyText?: string | null;
  advisory?: ContentAdvisoryView | null;
  /// メディア枠が無い advisory 付き投稿で、本文欄から詳細 dialog を開く。
  onOpenDetails?: (trigger: HTMLElement) => void;
};

function gatedContentDescription(
  t: TFunction,
  gatedBy: PostGatedContentProps['gatedBy']
): string {
  return gatedBy === 'advisory' ? t('feed.advisoryContentHidden') : t('feed.adultContentHidden');
}

export function PostGatedContent({
  objectId,
  gatedBy,
  bodyText,
  advisory,
  onOpenDetails,
}: PostGatedContentProps) {
  const { t } = useTranslation(['common']);

  if (!advisory) {
    return (
      <p
        className='topic-diagnostic topic-diagnostic-secondary'
        role='status'
        data-testid={`post-adult-gated-${objectId}`}
      >
        {bodyText ?? gatedContentDescription(t, gatedBy)}
      </p>
    );
  }

  return (
    <>
      {bodyText ? (
        <p
          className='topic-diagnostic topic-diagnostic-secondary'
          role='status'
          data-testid={`post-adult-gated-${objectId}`}
        >
          {bodyText}
        </p>
      ) : null}
      {onOpenDetails ? (
        <button
          type='button'
          className='post-advisory-trigger'
          aria-haspopup='dialog'
          data-testid={`post-advisory-details-trigger-${objectId}`}
          onClick={(event) => onOpenDetails(event.currentTarget)}
        >
          {t('feed.advisoryDetailsPost')}
        </button>
      ) : null}
    </>
  );
}

export function PostAdvisoryNotice({ advisory, objectId }: PostAdvisoryNoticeProps) {
  const { t } = useTranslation(['common']);
  const issuerLabel = advisoryIssuerLabel(advisory);

  return (
    <div className='post-advisory-note' data-testid={`post-advisory-gated-${objectId}`}>
      <p className='topic-diagnostic topic-diagnostic-secondary'>
        {t('advisory.description', { node: issuerLabel })}
      </p>
      <dl className='post-advisory-facts'>
        <div>
          <dt>{t('advisory.issuer')}</dt>
          <dd data-testid={`post-advisory-issuer-${objectId}`}>
            {issuerLabel}
            <span className='post-advisory-issuer-id'>
              {shortenNodeId(advisory.issuerNodeId)}
            </span>
          </dd>
        </div>
        <div>
          <dt>{t('advisory.category')}</dt>
          <dd>{advisoryCategoryLabel(t, advisory.category)}</dd>
        </div>
        {typeof advisory.confidence === 'number' ? (
          <div>
            <dt>{t('advisory.confidence')}</dt>
            <dd>{t('advisory.confidenceValue', { value: advisory.confidence })}</dd>
          </div>
        ) : null}
        <div>
          <dt>{t('advisory.basis')}</dt>
          <dd>{advisoryBasisLabel(t, advisory.basis)}</dd>
        </div>
      </dl>
    </div>
  );
}

/// #1108: advisory 付き投稿の詳細 dialog。一覧の代替表示から開き、従来カード内に展開していた
/// 説明文・推定の詳細・異議申し立ての導線をまとめて示す。メディアは描画しない(bytes を取得しない)。
export type PostAdvisoryDetailsDialogProps = {
  /// 開閉状態と、閉じたときに focus を戻す要素(開く操作をした代替表示)。
  details: ReturnType<typeof usePostAdvisoryDetails>;
  objectId: string;
  gatedBy?: PostGatedContentProps['gatedBy'];
  advisory: ContentAdvisoryView;
  /// 異議申し立て(通報 dialog)を開く。通報操作が出せない文脈では未指定にして導線を出さない。
  onAppeal?: () => void;
};

export function PostAdvisoryDetailsDialog({
  details,
  objectId,
  gatedBy,
  advisory,
  onAppeal,
}: PostAdvisoryDetailsDialogProps) {
  const { t } = useTranslation(['common']);
  const titleRef = useRef<HTMLHeadingElement>(null);

  return (
    <Dialog open={details.open} onOpenChange={details.setOpen}>
      <DialogContent
        className='post-advisory-dialog'
        data-testid={`post-advisory-dialog-${objectId}`}
        onOpenAutoFocus={(event) => {
          // 最初の操作(異議申し立て)へ focus を置くと、開いた直後の Enter で誤って進むため見出しに置く。
          event.preventDefault();
          titleRef.current?.focus();
        }}
        onCloseAutoFocus={(event) => {
          // 開く操作は DialogTrigger ではないため、元の代替表示へ明示的に戻す。
          // 異議申し立てへ移った場合は、通報 dialog を閉じた後に戻す。
          event.preventDefault();
          if (details.skipReturnFocus()) return;
          details.triggerRef.current?.focus();
        }}
      >
        <DialogHeader>
          <DialogTitle ref={titleRef} tabIndex={-1}>
            {t('advisory.title')}
          </DialogTitle>
          <DialogDescription>{gatedContentDescription(t, gatedBy)}</DialogDescription>
        </DialogHeader>
        <DialogBody>
          <PostAdvisoryNotice advisory={advisory} objectId={objectId} />
        </DialogBody>
        {onAppeal ? (
          <DialogFooter>
            <Button
              variant='secondary'
              type='button'
              data-testid={`post-advisory-appeal-${objectId}`}
              onClick={() => {
                details.handOffToReport();
                onAppeal();
              }}
            >
              {t('advisory.appeal')}
            </Button>
          </DialogFooter>
        ) : null}
      </DialogContent>
    </Dialog>
  );
}
