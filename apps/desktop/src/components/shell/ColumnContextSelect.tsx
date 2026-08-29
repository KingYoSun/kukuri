import { Select } from '@/components/ui/select';

export type ColumnContextSelectOption = {
  value: string;
  label: string;
  disabled?: boolean;
};

type ColumnContextSelectProps = {
  label: string;
  value: string;
  title?: string;
  options: readonly ColumnContextSelectOption[];
  onChange: (value: string) => void;
};

export function ColumnContextSelect({
  label,
  value,
  title,
  options,
  onChange,
}: ColumnContextSelectProps) {
  return (
    <label className='shell-column-context-control' data-column-preserve-activation>
      <span className='sr-only'>{label}</span>
      <Select
        className='shell-column-context-select'
        aria-label={label}
        value={value}
        title={title}
        onChange={(event) => onChange(event.currentTarget.value)}
      >
        {options.map((option) => (
          <option key={option.value} value={option.value} disabled={option.disabled}>
            {option.label}
          </option>
        ))}
      </Select>
    </label>
  );
}
