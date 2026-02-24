import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Antenna } from 'lucide-react';
import { RelayStatus } from '@/components/RelayStatus';
import { P2PStatus } from '@/components/P2PStatus';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

export function AntennaStatusDialog() {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [isOnline, setIsOnline] = useState(() => navigator.onLine);
  const modalTitle = `${t('relayStatus.title')} / ${t('p2pStatus.title')}`;
  const connectionStatusLabel = `${t('syncStatus.connectionStatus')}: ${isOnline ? t('syncStatus.online') : t('syncStatus.offline')}`;
  const buttonAriaLabel = `${modalTitle} (${connectionStatusLabel})`;

  useEffect(() => {
    const handleOnline = () => setIsOnline(true);
    const handleOffline = () => setIsOnline(false);

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);

    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
    };
  }, []);

  return (
    <>
      <Button
        variant="ghost"
        size="icon"
        aria-label={buttonAriaLabel}
        title={buttonAriaLabel}
        data-testid="open-network-status-button"
        onClick={() => setOpen(true)}
      >
        <Antenna
          className={`h-5 w-5 ${isOnline ? 'text-green-500' : 'text-red-500'}`}
          data-testid="network-status-antenna-icon"
        />
        <span className="sr-only">{connectionStatusLabel}</span>
      </Button>

      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent
          className="max-h-[calc(100vh-4rem)] overflow-hidden p-0 sm:max-w-5xl"
          data-testid="network-status-modal"
        >
          <div className="flex h-full max-h-[calc(100vh-4rem)] flex-col">
            <DialogHeader className="border-b px-6 py-4">
              <DialogTitle>{modalTitle}</DialogTitle>
              <DialogDescription>{t('p2pStatus.description')}</DialogDescription>
            </DialogHeader>

            <div className="grid gap-4 overflow-y-auto p-6 lg:grid-cols-2">
              <RelayStatus />
              <P2PStatus />
            </div>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}
