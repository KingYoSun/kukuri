import type * as React from 'react';

import { Field } from '@/components/ui/field';

type SettingsEditorFieldProps = {
  label: string;
  hint?: string;
  message?: string;
  tone?: 'default' | 'danger';
  children: React.ReactNode;
};

export function SettingsEditorField({
  label,
  hint,
  message,
  tone = 'default',
  children,
}: SettingsEditorFieldProps) {
  return (
    <Field label={label} hint={hint} message={message} tone={tone} className='gap-3'>
      {children}
    </Field>
  );
}
