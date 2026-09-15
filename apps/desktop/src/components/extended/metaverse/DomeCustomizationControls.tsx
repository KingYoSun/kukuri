import { useEffect, useRef, useState, type ReactNode } from 'react';
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
  renderSections?: (sections: { settings: ReactNode; objects: ReactNode; actions: ReactNode }) => ReactNode;
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
  renderSections,
}: DomeCustomizationControlsProps) {
  const { t } = useTranslation('metaverse', { lng: locale });
  const [draft, setDraft] = useState(() => cloneCustomization(customization));
  const [baseline, setBaseline] = useState(() => JSON.stringify(customization));
  const observed = useRef(baseline);
  const dirty = JSON.stringify(draft) !== baseline;
  const conflict = dirty && JSON.stringify(customization) !== baseline;
  const alive = useRef(true);
  const importVersion = useRef(0);
  useEffect(() => {
    alive.current = true;
    const version = importVersion.current;
    return () => { alive.current = false; importVersion.current = version + 1; };
  }, []);
  const [feedback, setFeedback] = useState<'idle' | 'saving' | 'saved' | 'error' | 'invalid'>('idle');

  useEffect(() => {
    const value = JSON.stringify(customization);
    if (value === observed.current) return;
    observed.current = value;
    if (!dirty) {
      setDraft(cloneCustomization(customization));
      setBaseline(JSON.stringify(customization));
      setFeedback('idle');
    }
  }, [customization, dirty]);

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
    const version = importVersion.current;
    try {
      const asset = await onImportTexture(file);
      if (!alive.current || version !== importVersion.current) return;
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
    if (!isOwner || pending || conflict) return;
    if (!Object.values(draft.environment).every(Number.isFinite) || !isDomeCustomizationValid(draft)) {
      setFeedback('invalid');
      return;
    }
    setFeedback('saving');
    try {
      await onSave(draft);
      if (!alive.current) return;
      setBaseline(JSON.stringify(draft));
      setFeedback('saved');
    } catch {
      setFeedback('error');
    }
  }

  const settings = !isOwner ? (
      <section className='metaverse-dome-customization' aria-label={t('customization.title')}>
        <strong>{t('customization.title')}</strong>
        <span>{t('customization.readOnly')}</span>
        <small>{t('customization.gravitySummary', { value: customization.environment.gravity_milli / 1_000 })}</small>
      </section>
  ) : <section className='metaverse-dome-customization' aria-label={t('customization.title')}>
      <strong>{t('customization.title')}</strong>
      <div className='metaverse-customization-grid'>
        {(['wall', 'floor'] as const).map((target) => (
          <Label key={target}>
            <span>{t(`customization.${target}Material`)}</span>
            <select
              aria-label={t(`customization.${target}Material`)}
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
        {([
          ['key_light_milli', 'keyLight', 1000, 0, 4],
          ['ambient_light_milli', 'ambient', 1000, 0, 2],
          ['fog_density_micros', 'fog', 2000, 0, 100],
          ['gravity_milli', 'gravity', 1000, 1, 30],
        ] as const).map(([field, label, factor, min, max]) => <Label key={field}>
          <span>{t(`customization.${label}`)}</span>
          <Input type='number' min={min} max={max} step='any' aria-label={t(`customization.${label}`)}
            value={Number.isFinite(draft.environment[field]) ? draft.environment[field] / factor : ''}
            disabled={pending}
            onChange={event => updateEnvironment(field, event.target.value === '' ? Number.NaN : Math.round(Number(event.target.value) * factor))} />
          <input type='range' aria-label={t(`customization.${label}`) + ' — ' + t('menu.slider')}
            min={min} max={max} step={1 / factor} disabled={pending}
            value={Number.isFinite(draft.environment[field]) ? draft.environment[field] / factor : min}
            onChange={event => updateEnvironment(field, Math.round(Number(event.target.value) * factor))} />
        </Label>)}
      </div>
    </section>;
  const objects = isOwner && firstProp ? (
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
      ) : null;
  const actions = isOwner ? <>
      {conflict && <p role='status'>{t('menu.conflict')}</p>}
      <div className='metaverse-customization-actions'>
        <Button type='button' size='sm' disabled={pending || feedback === 'saving' || conflict} onClick={() => void save()}>
          <Save className='size-4' aria-hidden='true' />
          {t('customization.save')}
        </Button>
        <Button type='button' size='sm' variant='secondary' disabled={pending} onClick={() => {
          importVersion.current++;
          setDraft(cloneCustomization(customization));
          setBaseline(JSON.stringify(customization));
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
    </> : null;
  return renderSections ? renderSections({ settings, objects, actions }) : <>{settings}{objects}{actions}</>;
}
