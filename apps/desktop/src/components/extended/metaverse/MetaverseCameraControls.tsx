import { useTranslation } from 'react-i18next';
import type { SupportedLocale } from '@/i18n';
import { Button } from '@/components/ui/button';
import type { CameraCommand } from './MetaverseCameraModel';
import type { SceneInputMode } from './useMetaverseSceneInput';

export function MetaverseCameraControls({ locale, mode, enabled, onStart, onCommand }: {
  locale: SupportedLocale;
  mode: SceneInputMode;
  enabled: boolean;
  onStart: () => void;
  onCommand: (command: CameraCommand) => void;
}) {
  const { t } = useTranslation('metaverse', { lng: locale });
  return <div className='metaverse-camera-controls' data-metaverse-ui>
    <span role='status'>{t(`camera.states.${enabled ? mode : 'inactive'}`)}</span>
    {mode === 'locked' ? <span>{t('camera.hint')}</span> : <>
      <Button size='sm' type='button' disabled={!enabled || mode === 'requesting'} onClick={onStart}>{t('camera.resume')}</Button>
      <details>
        <summary>{t('camera.controls')}</summary>
        <div className='metaverse-camera-buttons'>
          {(['left', 'right', 'up', 'down', 'in', 'out', 'reset'] as const).map(command =>
            <Button key={command} size='sm' variant='secondary' type='button' disabled={!enabled} onClick={() => onCommand(command)}>{t(`camera.${command}`)}</Button>)}
        </div>
      </details>
    </>}
  </div>;
}
