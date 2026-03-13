import { Toaster as Sonner, ToasterProps } from 'sonner';
import { useUIStore } from '@/stores/uiStore';

const Toaster = ({ ...props }: ToasterProps) => {
  const theme = useUIStore((state) => state.theme);

  // Map theme to sonner theme format
  const sonnerTheme = theme === 'dark' ? 'dark' : theme === 'light' ? 'light' : 'system';

  return (
    <Sonner
      theme={sonnerTheme as ToasterProps['theme']}
      className="toaster group"
      style={
        {
          '--normal-bg': 'var(--popover)',
          '--normal-text': 'var(--popover-foreground)',
          '--normal-border': 'var(--border)',
        } as React.CSSProperties
      }
      {...props}
    />
  );
};

export { Toaster };
