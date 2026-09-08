import { useRef } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Dialog, DialogBody, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';

export function CommunityNodeOnboardingDialog({
  baseUrl, nodeLabel, onDismiss, onReview, onOpenSettings, onCloseAutoFocus,
}: {
  baseUrl: string;
  nodeLabel: string;
  onDismiss: () => void;
  onReview: () => void;
  onOpenSettings: () => void;
  onCloseAutoFocus?: (event: Event) => void;
}) {
  const { t } = useTranslation(['settings', 'common']);
  const title = useRef<HTMLHeadingElement>(null);
  return (
    <Dialog open onOpenChange={(open) => { if (!open) onDismiss(); }}>
      <DialogContent
        className='max-h-[88vh] w-[min(32rem,92vw)] overflow-y-auto'
        onOpenAutoFocus={(event) => { event.preventDefault(); title.current?.focus(); }}
        onCloseAutoFocus={onCloseAutoFocus}
      >
        <DialogHeader>
          <DialogTitle ref={title} tabIndex={-1}>{t('settings:communityNode.onboarding.title')}</DialogTitle>
          <DialogDescription>{t('settings:communityNode.onboarding.description')}</DialogDescription>
        </DialogHeader>
        <DialogBody className='space-y-3'>
          <p className='break-words'>{t('settings:communityNode.onboarding.candidate', { node: nodeLabel })}</p>
          <details className='text-sm text-[var(--muted-foreground)]'>
            <summary>{t('settings:communityNode.onboarding.address')}</summary>
            <p className='break-all'>{baseUrl}</p>
          </details>
          <Button variant='ghost' onClick={onOpenSettings}>{t('settings:communityNode.onboarding.openSettings')}</Button>
        </DialogBody>
        <DialogFooter className='flex flex-wrap justify-end gap-2'>
          <Button variant='secondary' onClick={onDismiss}>{t('settings:communityNode.onboarding.later')}</Button>
          <Button onClick={onReview}>{t('settings:communityNode.onboarding.reviewPolicies')}</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
