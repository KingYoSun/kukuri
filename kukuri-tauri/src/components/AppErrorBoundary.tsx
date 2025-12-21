import React from 'react';
import { errorHandler } from '@/lib/errorHandler';

type AppErrorBoundaryProps = {
  children: React.ReactNode;
};

type AppErrorBoundaryState = {
  hasError: boolean;
  error: Error | null;
};

const clampText = (value: string, max = 500) => (value.length > max ? value.slice(0, max) : value);

export class AppErrorBoundary extends React.Component<
  AppErrorBoundaryProps,
  AppErrorBoundaryState
> {
  state: AppErrorBoundaryState = { hasError: false, error: null };

  static getDerivedStateFromError(error: Error): AppErrorBoundaryState {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    errorHandler.log('React render failed', error, {
      context: 'AppErrorBoundary',
      metadata: { componentStack: info.componentStack },
    });
    if (typeof document !== 'undefined' && document.documentElement) {
      const message = error.stack ?? error.message ?? 'Unknown error';
      const componentStack = info.componentStack?.trim();
      const summary = message.split('\n').slice(0, 3).join('\n');
      const combined = componentStack ? `${componentStack}\n${summary}` : summary;
      document.documentElement.setAttribute('data-kukuri-e2e-error', clampText(combined));
    }
  }

  render() {
    if (!this.state.hasError) {
      return this.props.children;
    }

    return (
      <div className="p-6" data-testid="app-error-boundary">
        <h1 className="text-lg font-semibold">Something went wrong.</h1>
        {this.state.error ? (
          <p className="mt-2 text-sm text-muted-foreground">{this.state.error.message}</p>
        ) : null}
      </div>
    );
  }
}
