import { useEffect, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import {
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Label,
  Notice,
  Textarea
} from '../components/ui';
import { StatusBadge } from '../components/StatusBadge';
import { api } from '../lib/api';
import { errorToMessage } from '../lib/errorHandler';
import { formatJson, formatTimestamp } from '../lib/format';
import type { ServiceInfo } from '../lib/types';

const SECRET_CONFIG_PATH_PREVIEW_LIMIT = 3;

const tokenizeConfigKey = (key: string): string[] => {
  const chars = Array.from(key);
  const tokens: string[] = [];
  let current = '';

  for (let index = 0; index < chars.length; index += 1) {
    const ch = chars[index];
    if (/^[a-zA-Z0-9]$/.test(ch)) {
      const prev = index > 0 ? chars[index - 1] : null;
      const next = index + 1 < chars.length ? chars[index + 1] : null;
      const boundary =
        current !== '' &&
        ch >= 'A' &&
        ch <= 'Z' &&
        ((prev !== null && prev >= 'a' && prev <= 'z') ||
          (prev !== null &&
            prev >= 'A' &&
            prev <= 'Z' &&
            next !== null &&
            next >= 'a' &&
            next <= 'z'));

      if (boundary) {
        tokens.push(current);
        current = '';
      }
      current += ch.toLowerCase();
    } else if (current !== '') {
      tokens.push(current);
      current = '';
    }
  }

  if (current !== '') {
    tokens.push(current);
  }

  return tokens;
};

const containsTokenPair = (tokens: string[], left: string, right: string): boolean =>
  tokens.includes(left) && tokens.includes(right);

const isSecretLikeConfigKey = (key: string): boolean => {
  const tokens = tokenizeConfigKey(key);
  if (tokens.length === 0) {
    return false;
  }

  const forbiddenTokens = new Set([
    'secret',
    'password',
    'passwd',
    'pwd',
    'apikey',
    'secretkey',
    'privatekey',
    'masterkey',
    'accesskey',
    'clientsecret',
    'jwtsecret',
    'authsecret',
    'signingkey',
    'encryptionkey',
    'hmacsecret'
  ]);

  if (tokens.some((token) => forbiddenTokens.has(token))) {
    return true;
  }

  return (
    containsTokenPair(tokens, 'api', 'key') ||
    containsTokenPair(tokens, 'private', 'key') ||
    containsTokenPair(tokens, 'master', 'key') ||
    containsTokenPair(tokens, 'access', 'key') ||
    containsTokenPair(tokens, 'client', 'secret') ||
    containsTokenPair(tokens, 'jwt', 'secret') ||
    containsTokenPair(tokens, 'auth', 'secret') ||
    containsTokenPair(tokens, 'signing', 'key') ||
    containsTokenPair(tokens, 'encryption', 'key') ||
    containsTokenPair(tokens, 'hmac', 'secret')
  );
};

const escapeJsonPointerToken = (token: string): string =>
  token.replace(/~/g, '~0').replace(/\//g, '~1');

const collectSecretConfigPaths = (value: unknown, currentPath = ''): string[] => {
  if (Array.isArray(value)) {
    return value.flatMap((item, index) =>
      collectSecretConfigPaths(item, `${currentPath}/${index.toString()}`)
    );
  }

  if (value === null || typeof value !== 'object') {
    return [];
  }

  return Object.entries(value).flatMap(([key, nested]) => {
    const path = `${currentPath}/${escapeJsonPointerToken(key)}`;
    const matched = isSecretLikeConfigKey(key) ? [path] : [];
    return [...matched, ...collectSecretConfigPaths(nested, path)];
  });
};

const buildSecretConfigErrorMessage = (paths: string[]): string => {
  const preview = paths.slice(0, SECRET_CONFIG_PATH_PREVIEW_LIMIT).join(', ');
  if (paths.length > SECRET_CONFIG_PATH_PREVIEW_LIMIT) {
    return `Secret keys are not allowed in service config: ${preview} and ${
      paths.length - SECRET_CONFIG_PATH_PREVIEW_LIMIT
    } more. Use environment secrets instead.`;
  }
  return `Secret keys are not allowed in service config: ${preview}. Use environment secrets instead.`;
};

const ServiceCard = ({ service }: { service: ServiceInfo }) => {
  const queryClient = useQueryClient();
  const [draft, setDraft] = useState(formatJson(service.config_json));
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    setDraft(formatJson(service.config_json));
  }, [service.version, service.config_json]);

  const mutation = useMutation({
    mutationFn: (payload: unknown) =>
      api.updateServiceConfig(service.service, payload, service.version),
    onSuccess: () => {
      setMessage('Saved.');
      queryClient.invalidateQueries({ queryKey: ['services'] });
    },
    onError: (err) => {
      setMessage(errorToMessage(err));
    }
  });

  const save = () => {
    setMessage(null);
    try {
      const parsed = JSON.parse(draft);
      const secretPaths = Array.from(new Set(collectSecretConfigPaths(parsed))).sort();
      if (secretPaths.length > 0) {
        setMessage(buildSecretConfigErrorMessage(secretPaths));
        return;
      }
      mutation.mutate(parsed);
    } catch {
      setMessage('Config must be valid JSON.');
    }
  };

  return (
    <Card>
      <CardHeader className="row">
        <div>
          <CardTitle>{service.service}</CardTitle>
          <p>Version {service.version}</p>
        </div>
        <StatusBadge status={service.health?.status ?? 'unknown'} />
      </CardHeader>
      <CardContent>
        <div className="muted">
          Updated {formatTimestamp(service.updated_at)} by {service.updated_by}
        </div>
        <div className="field">
          <Label htmlFor={`${service.service}-config`}>Config JSON</Label>
          <Textarea
            id={`${service.service}-config`}
            rows={10}
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
          />
        </div>
        {message && <Notice tone={message === 'Saved.' ? 'success' : 'error'}>{message}</Notice>}
        <Button onClick={save} disabled={mutation.isPending}>
          {mutation.isPending ? 'Saving...' : 'Save config'}
        </Button>
      </CardContent>
    </Card>
  );
};

export const ServicesPage = () => {
  const { data, isLoading, error } = useQuery<ServiceInfo[]>({
    queryKey: ['services'],
    queryFn: api.services
  });

  return (
    <>
      <div className="hero">
        <div>
          <h1>Services</h1>
          <p>Update runtime configuration and monitor status.</p>
        </div>
      </div>
      {isLoading && <Notice>Loading services...</Notice>}
      {error && <Notice tone="error">{errorToMessage(error)}</Notice>}
      <div className="grid">
        {(data ?? []).map((service) => (
          <ServiceCard key={service.service} service={service} />
        ))}
      </div>
    </>
  );
};
