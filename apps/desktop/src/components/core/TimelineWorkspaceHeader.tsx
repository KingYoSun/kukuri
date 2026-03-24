import type * as React from 'react';

import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import { Select } from '@/components/ui/select';

type SelectOption = {
  value: string;
  label: string;
};

type TimelineWorkspaceHeaderProps = {
  activeTopic: string;
  viewingLabel: string;
  postingLabel: string;
  viewScopeValue: string;
  composeTargetValue: string;
  viewScopeOptions: SelectOption[];
  composeTargetOptions: SelectOption[];
  contextButtonRef?: React.RefObject<HTMLButtonElement | null>;
  contextOpen: boolean;
  contextControlsId: string;
  onOpenContext: () => void;
  onRefresh: () => void;
  onViewScopeChange: (value: string) => void;
  onComposeTargetChange: (value: string) => void;
  composeTargetDisabled?: boolean;
};

export function TimelineWorkspaceHeader({
  activeTopic,
  viewingLabel,
  postingLabel,
  viewScopeValue,
  composeTargetValue,
  viewScopeOptions,
  composeTargetOptions,
  contextButtonRef,
  contextOpen,
  contextControlsId,
  onOpenContext,
  onRefresh,
  onViewScopeChange,
  onComposeTargetChange,
  composeTargetDisabled,
}: TimelineWorkspaceHeaderProps) {
  return (
    <>
      <div className='shell-workspace-header'>
        <div>
          <h2>Timeline</h2>
          <span className='active-topic-label'>{activeTopic}</span>
        </div>
        <div className='shell-inline-actions'>
          <div className='shell-workspace-summary'>
            <span className='relationship-badge'>{`viewing ${viewingLabel.toLowerCase()}`}</span>
            <span className='relationship-badge relationship-badge-direct'>
              {`posting ${postingLabel.toLowerCase()}`}
            </span>
          </div>
          <Button
            ref={contextButtonRef}
            className='shell-context-trigger'
            variant='ghost'
            type='button'
            aria-label='Open context pane'
            aria-controls={contextControlsId}
            aria-expanded={contextOpen}
            data-testid='shell-context-trigger'
            onClick={onOpenContext}
          >
            Open Context
          </Button>
          <Button variant='secondary' onClick={onRefresh}>
            Refresh
          </Button>
        </div>
      </div>

      <div className='topic-diagnostic'>
        <Label>
          <span>View Scope</span>
          <Select
            aria-label='View Scope'
            value={viewScopeValue}
            onChange={(event) => onViewScopeChange(event.target.value)}
          >
            {viewScopeOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </Select>
        </Label>

        <Label>
          <span>Compose Target</span>
          <Select
            aria-label='Compose Target'
            value={composeTargetValue}
            disabled={composeTargetDisabled}
            onChange={(event) => onComposeTargetChange(event.target.value)}
          >
            {composeTargetOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </Select>
        </Label>
      </div>

      <div className='topic-diagnostic topic-diagnostic-secondary'>
        <span>Viewing: {viewingLabel}</span>
        <span>Posting to: {postingLabel}</span>
      </div>
    </>
  );
}
