import { cn } from '@/lib/utils';
import { type PrimarySection } from '@/components/shell/types';

type WorkspaceTab = {
  id: PrimarySection;
  label: string;
};

type TimelineWorkspaceHeaderProps = {
  activeSection: PrimarySection;
  items: WorkspaceTab[];
  onSelectSection: (section: PrimarySection) => void;
};

export function TimelineWorkspaceHeader({
  activeSection,
  items,
  onSelectSection,
}: TimelineWorkspaceHeaderProps) {
  return (
    <div className='shell-workspace-tabs' role='tablist' aria-label='Workspaces'>
      {items.map((item) => (
        <button
          key={item.id}
          className={cn('shell-tab', activeSection === item.id && 'shell-tab-active')}
          id={`workspace-tab-${item.id}`}
          role='tab'
          type='button'
          aria-selected={activeSection === item.id}
          tabIndex={activeSection === item.id ? 0 : -1}
          onClick={() => onSelectSection(item.id)}
        >
          {item.label}
        </button>
      ))}
    </div>
  );
}
