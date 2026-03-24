import { Button } from '@/components/ui/button';

import { type ComposerDraftMediaView } from './types';

type ComposerDraftPreviewListProps = {
  items: ComposerDraftMediaView[];
  onRemove: (itemId: string) => void;
};

export function ComposerDraftPreviewList({ items, onRemove }: ComposerDraftPreviewListProps) {
  if (items.length === 0) {
    return null;
  }

  return (
    <ul className='draft-attachment-list'>
      {items.map((item) => (
        <li key={item.id} className='draft-attachment-item'>
          <div className='draft-attachment-content'>
            <div className='draft-preview-frame'>
              <img className='draft-preview-image' src={item.previewUrl} alt={`draft preview ${item.sourceName}`} />
            </div>
            <div>
              <strong>{item.sourceName}</strong>
              {item.attachments.map((attachment) => (
                <small key={attachment.key}>
                  {attachment.label} · {attachment.mime} · {attachment.byteSizeLabel}
                </small>
              ))}
            </div>
          </div>
          <Button variant='secondary' type='button' onClick={() => onRemove(item.id)}>
            Remove
          </Button>
        </li>
      ))}
    </ul>
  );
}
