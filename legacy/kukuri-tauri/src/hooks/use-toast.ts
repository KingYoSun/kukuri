import { toast as sonnerToast } from 'sonner';

interface ToastOptions {
  title?: string;
  description?: string;
  variant?: 'default' | 'destructive';
}

export function useToast() {
  const toast = (options: ToastOptions) => {
    const { title, description, variant } = options;
    const message = title || description || '';

    if (variant === 'destructive') {
      sonnerToast.error(message, {
        description: title && description ? description : undefined,
      });
    } else {
      sonnerToast.success(message, {
        description: title && description ? description : undefined,
      });
    }
  };

  return { toast };
}
