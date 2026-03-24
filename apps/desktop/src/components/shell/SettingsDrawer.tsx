import * as React from 'react';

import { X } from 'lucide-react';

import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { cn } from '@/lib/utils';

import { type SettingsSection } from './types';

type SettingsDrawerSection = {
  id: SettingsSection;
  label: string;
  description: string;
  content: React.ReactNode;
};

type SettingsDrawerProps = {
  drawerId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  activeSection: SettingsSection;
  onSectionChange: (section: SettingsSection) => void;
  sections: SettingsDrawerSection[];
};

export function SettingsDrawer({
  drawerId,
  open,
  onOpenChange,
  activeSection,
  onSectionChange,
  sections,
}: SettingsDrawerProps) {
  const currentSection = sections.find((section) => section.id === activeSection) ?? sections[0];

  return (
    <>
      <div
        className='shell-overlay-backdrop shell-settings-backdrop'
        data-open={open}
        onClick={() => onOpenChange(false)}
        aria-hidden='true'
      />
      <Card
        as='section'
        id={drawerId}
        className='shell-settings-drawer'
        data-open={open}
        role='dialog'
        aria-modal='true'
        aria-hidden={!open}
        aria-labelledby={`${drawerId}-title`}
      >
        <div className='shell-settings-nav'>
          <div className='shell-pane-header shell-pane-header-compact'>
            <div>
              <p className='eyebrow'>Settings</p>
              <h2 id={`${drawerId}-title`} className='shell-pane-heading'>
                Settings & diagnostics
              </h2>
            </div>
            <Button
              variant='ghost'
              size='icon'
              type='button'
              aria-label='Close settings and diagnostics'
              onClick={() => onOpenChange(false)}
            >
              <X className='size-4' aria-hidden='true' />
            </Button>
          </div>
          <nav aria-label='Settings sections' className='shell-settings-nav-list'>
            {sections.map((section) => (
              <button
                key={section.id}
                className={cn(
                  'shell-settings-nav-item',
                  activeSection === section.id && 'shell-settings-nav-item-active'
                )}
                type='button'
                aria-current={activeSection === section.id ? 'location' : undefined}
                data-testid={`settings-section-${section.id}`}
                onClick={() => onSectionChange(section.id)}
              >
                <span className='shell-primary-nav-label'>{section.label}</span>
                <span className='shell-primary-nav-copy'>{section.description}</span>
              </button>
            ))}
          </nav>
        </div>

        <div className='shell-settings-body'>
          <div className='shell-settings-current'>
            <p className='eyebrow'>Current section</p>
            <h3 className='shell-pane-heading'>{currentSection.label}</h3>
            <p className='shell-pane-copy'>{currentSection.description}</p>
          </div>
          <div className='shell-settings-content'>{currentSection.content}</div>
        </div>
      </Card>
    </>
  );
}
