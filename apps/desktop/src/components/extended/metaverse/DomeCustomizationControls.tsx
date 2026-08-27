import { useEffect, useState } from 'react';
import { Save, Undo2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { SupportedLocale } from '@/i18n';
import type {
  DomeCustomizationV1,
  DomeMaterialPreset,
  MetaverseAssetRef,
  MetaverseInteractionKind,
} from '@/lib/api';
import { DOME_INTERACTIONS, isDomeCustomizationValid } from './DomeSceneModel';

type TextureTarget = 'wall' | 'floor';

type DomeCustomizationControlsProps = {
  customization: DomeCustomizationV1;
  isOwner: boolean;
  pending: boolean;
  locale: SupportedLocale;
  onSave: (customization: DomeCustomizationV1) => Promise<void>;
  onImportTexture: (file: File) => Promise<MetaverseAssetRef>;
};

const MATERIAL_PRESETS: DomeMaterialPreset[] = ['concrete', 'stone', 'metal', 'wood'];

function cloneCustomization(value: DomeCustomizationV1): DomeCustomizationV1 {
  return JSON.parse(JSON.stringify(value)) as DomeCustomizationV1;
}

export function DomeCustomizationControls({
  customization,
  isOwner,
  pending,
  locale,
  onSave,
  onImportTexture,
}: DomeCustomizationControlsProps) {
  const { t } = useTranslation('metaverse', { lng: locale });
  const [draft, setDraft] = useState(() => cloneCustomization(customization));
  const [feedback, setFeedback] = useState<'idle' | 'saving' | 'saved' | 'error' | 'invalid'>('idle');

  useEffect(() => {
    setDraft(cloneCustomization(customization));
    setFeedback('idle');
  }, [customization]);

  const firstProp = draft.persistent_props[0];

  function updateMaterial(target: TextureTarget, material: DomeMaterialPreset) {
    setDraft((current) => ({
      ...current,
      surface: {
        ...current.surface,
        [target === 'wall' ? 'wall_material' : 'floor_material']: material,
      },
    }));
    setFeedback('idle');
  }

  function updateEnvironment(field: keyof DomeCustomizationV1['environment'], value: number) {
    setDraft((current) => ({
      ...current,
      environment: { ...current.environment, [field]: value },
    }));
    setFeedback('idle');
  }

  function updateFirstProp(updater: (prop: NonNullable<typeof firstProp>) => NonNullable<typeof firstProp>) {
    if (!firstProp) return;
    setDraft((current) => ({
      ...current,
      persistent_props: current.persistent_props.map((prop, index) => index === 0 ? updater(prop) : prop),
    }));
    setFeedback('idle');
  }

  function toggleInteraction(interaction: MetaverseInteractionKind) {
    updateFirstProp((prop) => ({
      ...prop,
      visual_only: false,
      interactions: prop.interactions.includes(interaction)
        ? prop.interactions.filter((value) => value !== interaction)
        : [...prop.interactions, interaction],
    }));
  }

  async function importTexture(target: TextureTarget, file: File) {
    try {
      const asset = await onImportTexture(file);
      setDraft((current) => ({
        ...current,
        surface: {
          ...current.surface,
          [target === 'wall' ? 'wall_texture' : 'floor_texture']: asset,
        },
      }));
      setFeedback('idle');
    } catch {
      setFeedback('error');
    }
  }

  async function save() {
    if (!isDomeCustomizationValid(draft)) {
      setFeedback('invalid');
      return;
    }
    setFeedback('saving');
    try {
      await onSave(draft);
      setFeedback('saved');
    } catch {
      setFeedback('error');
    }
  }

  if (!isOwner) {
    return (
      <section className='metaverse-dome-customization' aria-label={t('customization.title')}>
        <strong>{t('customization.title')}</strong>
        <span>{t('customization.readOnly')}</span>
        <small>{t('customization.gravitySummary', { value: customization.environment.gravity_milli / 1_000 })}</small>
      </section>
    );
  }

  return (
    <section className='metaverse-dome-customization' aria-label={t('customization.title')}>
      <strong>{t('customization.title')}</strong>
      <div className='metaverse-customization-grid'>
        {(['wall', 'floor'] as const).map((target) => (
          <Label key={target}>
            <span>{t(`customization.${target}Material`)}</span>
            <select
              value={target === 'wall' ? draft.surface.wall_material : draft.surface.floor_material}
              disabled={pending}
              onChange={(event) => updateMaterial(target, event.target.value as DomeMaterialPreset)}
            >
              {MATERIAL_PRESETS.map((preset) => (
                <option key={preset} value={preset}>{t(`customization.materials.${preset}`)}</option>
              ))}
            </select>
          </Label>
        ))}
        {(['wall', 'floor'] as const).map((target) => (
          <Label key={`${target}-texture`}>
            <span>{t(`customization.${target}Texture`)}</span>
            <Input
              type='file'
              accept='image/png,image/jpeg,image/webp'
              disabled={pending}
              onChange={(event) => {
                const file = event.target.files?.[0];
                if (file) void importTexture(target, file);
                event.currentTarget.value = '';
              }}
            />
          </Label>
        ))}
        <Label>
          <span>{t('customization.keyLight')}</span>
          <Input type='number' min={0} max={4000} step={100} value={draft.environment.key_light_milli}
            disabled={pending}
            onChange={(event) => updateEnvironment('key_light_milli', Number(event.target.value))} />
        </Label>
        <Label>
          <span>{t('customization.ambient')}</span>
          <Input type='number' min={0} max={2000} step={100} value={draft.environment.ambient_light_milli}
            disabled={pending}
            onChange={(event) => updateEnvironment('ambient_light_milli', Number(event.target.value))} />
        </Label>
        <Label>
          <span>{t('customization.fog')}</span>
          <Input type='number' min={0} max={200000} step={1000} value={draft.environment.fog_density_micros}
            disabled={pending}
            onChange={(event) => updateEnvironment('fog_density_micros', Number(event.target.value))} />
        </Label>
        <Label>
          <span>{t('customization.gravity')}</span>
          <Input type='number' min={1000} max={30000} step={100} value={draft.environment.gravity_milli}
            disabled={pending}
            onChange={(event) => updateEnvironment('gravity_milli', Number(event.target.value))} />
        </Label>
      </div>
      {firstProp ? (
        <fieldset className='metaverse-interaction-options'>
          <legend>{t('customization.propInteractions')}</legend>
          {DOME_INTERACTIONS.map((interaction) => (
            <Label key={interaction}>
              <input
                type='checkbox'
                checked={firstProp.interactions.includes(interaction)}
                disabled={pending || firstProp.visual_only}
                onChange={() => toggleInteraction(interaction)}
              />
              <span>{t(`customization.interactions.${interaction}`)}</span>
            </Label>
          ))}
          <Label>
            <input
              type='checkbox'
              checked={firstProp.visual_only}
              disabled={pending}
              onChange={(event) => updateFirstProp((prop) => ({
                ...prop,
                visual_only: event.target.checked,
                interactions: event.target.checked ? [] : prop.interactions,
              }))}
            />
            <span>{t('customization.visualOnly')}</span>
          </Label>
        </fieldset>
      ) : null}
      <div className='metaverse-customization-actions'>
        <Button type='button' size='sm' disabled={pending || feedback === 'saving'} onClick={() => void save()}>
          <Save className='size-4' aria-hidden='true' />
          {t('customization.save')}
        </Button>
        <Button type='button' size='sm' variant='secondary' disabled={pending} onClick={() => {
          setDraft(cloneCustomization(customization));
          setFeedback('idle');
        }}>
          <Undo2 className='size-4' aria-hidden='true' />
          {t('customization.cancel')}
        </Button>
      </div>
      {feedback !== 'idle' ? (
        <span className='metaverse-customization-feedback' data-state={feedback} role='status'>
          {t(`customization.feedback.${feedback}`)}
        </span>
      ) : null}
    </section>
  );
}
