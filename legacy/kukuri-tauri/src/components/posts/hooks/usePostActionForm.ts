import { useCallback, useState } from 'react';
import type { QueryKey } from '@tanstack/react-query';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';

import { errorHandler } from '@/lib/errorHandler';

interface InvalidateTarget {
  queryKey: QueryKey;
}

interface UsePostActionFormOptions {
  submit: (content: string) => Promise<void>;
  successMessage: string;
  emptyErrorMessage: string;
  errorContext: string;
  errorToastTitle: string;
  invalidations?: Array<InvalidateTarget | null | undefined>;
  onSuccess?: () => void;
}

export const usePostActionForm = ({
  submit,
  successMessage,
  emptyErrorMessage,
  errorContext,
  errorToastTitle,
  invalidations = [],
  onSuccess,
}: UsePostActionFormOptions) => {
  const queryClient = useQueryClient();
  const [content, setContent] = useState('');

  const mutation = useMutation({
    mutationFn: async (value: string) => {
      const trimmed = value.trim();
      if (!trimmed) {
        throw new Error(emptyErrorMessage);
      }
      await submit(trimmed);
    },
    onSuccess: () => {
      setContent('');
      toast.success(successMessage);
      invalidations.forEach((target) => {
        if (!target) return;
        queryClient.invalidateQueries({ queryKey: target.queryKey });
      });
      onSuccess?.();
    },
    onError: (error: unknown) => {
      errorHandler.log('Post action failed', error, {
        context: errorContext,
        showToast: true,
        toastTitle: errorToastTitle,
      });
    },
  });

  const handleSubmit = useCallback(
    (event?: React.FormEvent) => {
      event?.preventDefault();
      if (!content.trim() || mutation.isPending) {
        return;
      }
      mutation.mutate(content);
    },
    [content, mutation],
  );

  const handleKeyboardSubmit = useCallback(
    (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (event.key === 'Enter' && (event.ctrlKey || event.metaKey)) {
        event.preventDefault();
        handleSubmit();
      }
    },
    [handleSubmit],
  );

  return {
    content,
    setContent,
    isPending: mutation.isPending,
    handleSubmit,
    handleKeyboardSubmit,
  };
};
