import { useState, useCallback, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { Search, X } from 'lucide-react';
import { useDebounce } from '@/hooks/useDebounce';
import { cn } from '@/lib/utils';

interface SearchBarProps {
  placeholder?: string;
  value?: string;
  onChange?: (value: string) => void;
  onSearch?: (query: string) => void;
  onClear?: () => void;
  className?: string;
  autoFocus?: boolean;
  showButton?: boolean;
  validationState?: 'default' | 'warning' | 'error';
  validationMessage?: string;
  helperLabel?: string;
}

export function SearchBar({
  placeholder,
  value: controlledValue,
  onChange,
  onSearch,
  onClear,
  className,
  autoFocus = false,
  showButton = true,
  validationState = 'default',
  validationMessage,
  helperLabel,
}: SearchBarProps) {
  const { t } = useTranslation();
  const defaultPlaceholder = placeholder ?? t('search.placeholder');
  const [internalValue, setInternalValue] = useState('');
  const value = controlledValue !== undefined ? controlledValue : internalValue;
  const debouncedValue = useDebounce(value, 300);

  // デバウンスされた値が変更されたときに検索を実行
  useEffect(() => {
    if (debouncedValue && onSearch) {
      onSearch(debouncedValue);
    }
  }, [debouncedValue, onSearch]);

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const newValue = e.target.value;
      if (controlledValue === undefined) {
        setInternalValue(newValue);
      }
      onChange?.(newValue);
    },
    [controlledValue, onChange],
  );

  const handleClear = useCallback(() => {
    if (controlledValue === undefined) {
      setInternalValue('');
    }
    onChange?.('');
    onClear?.();
    onSearch?.('');
  }, [controlledValue, onChange, onClear, onSearch]);

  const handleSubmit = useCallback(
    (e: React.FormEvent) => {
      e.preventDefault();
      if (value.trim() && onSearch) {
        onSearch(value.trim());
      }
    },
    [value, onSearch],
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === 'Escape' && value) {
        handleClear();
      }
    },
    [value, handleClear],
  );

  const inputValidationClass =
    validationState === 'error'
      ? 'border-destructive focus-visible:ring-destructive/70'
      : validationState === 'warning'
        ? 'border-amber-400 text-foreground focus-visible:ring-amber-400/70'
        : '';
  const messageClass =
    validationState === 'error'
      ? 'text-destructive'
      : validationState === 'warning'
        ? 'text-amber-600'
        : 'text-muted-foreground';

  return (
    <form onSubmit={handleSubmit} className={cn('flex gap-2', className)}>
      <div className="flex-1">
        <div className="relative">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <Input
            type="search"
            value={value}
            onChange={handleChange}
            onKeyDown={handleKeyDown}
            placeholder={defaultPlaceholder}
            className={cn('pl-9 pr-9', inputValidationClass)}
            autoFocus={autoFocus}
            aria-invalid={validationState === 'error'}
            data-testid="search-input"
          />
          {value && (
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="absolute right-1 top-1/2 h-6 w-6 -translate-y-1/2"
              onClick={handleClear}
              data-testid="search-clear"
            >
              <X className="h-3 w-3" />
              <span className="sr-only">{t('common.clear')}</span>
            </Button>
          )}
        </div>
        {(helperLabel || validationMessage) && (
          <div className="mt-1 space-y-0.5">
            {helperLabel && (
              <p className="text-xs text-muted-foreground" data-testid="search-helper-label">
                {helperLabel}
              </p>
            )}
            {validationMessage && (
              <p className={cn('text-xs', messageClass)} data-testid="search-validation-message">
                {validationMessage}
              </p>
            )}
          </div>
        )}
      </div>
      {showButton && (
        <Button type="submit" disabled={!value.trim()}>
          {t('search.search')}
        </Button>
      )}
    </form>
  );
}
