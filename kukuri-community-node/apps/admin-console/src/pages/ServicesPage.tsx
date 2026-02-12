import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import {
  Button,
  Card,
  CardContent,
  CardHeader,
  CardTitle,
  Input,
  Label,
  Notice,
  Select,
  Textarea
} from '../components/ui';
import { StatusBadge } from '../components/StatusBadge';
import { asRecord } from '../lib/config';
import { api } from '../lib/api';
import { errorToMessage } from '../lib/errorHandler';
import { formatJson, formatTimestamp } from '../lib/format';
import type { ServiceInfo } from '../lib/types';

const SECRET_CONFIG_PATH_PREVIEW_LIMIT = 3;
const AUTH_FORM_TARGETS = new Set(['relay', 'bootstrap']);

type AuthMode = 'off' | 'required';
type EnforceMode = 'immediate' | 'scheduled';

type AuthTransitionForm = {
  mode: AuthMode;
  enforceMode: EnforceMode;
  enforceAtLocal: string;
  graceSeconds: string;
  wsAuthTimeoutSeconds: string;
};

type AuthRuntimeStatus = {
  phase: string;
  status: string;
  enforceAt: number | null;
  disconnectAt: number | null;
  secondsUntilEnforce: number | null;
  secondsUntilDisconnect: number | null;
};

type RelayAuthMetrics = {
  metricsStatus: number | null;
  wsConnections: number | null;
  wsUnauthenticatedConnections: number | null;
  authRejectTotal: number | null;
  authTimeoutDisconnectTotal: number | null;
  authDeadlineDisconnectTotal: number | null;
  metricsError: string | null;
};

const toLocalDatetimeValue = (epochSeconds: number | null): string => {
  if (epochSeconds === null) {
    return '';
  }
  const date = new Date(epochSeconds * 1000);
  if (Number.isNaN(date.getTime())) {
    return '';
  }

  const pad = (value: number) => String(value).padStart(2, '0');
  const year = date.getFullYear();
  const month = pad(date.getMonth() + 1);
  const day = pad(date.getDate());
  const hours = pad(date.getHours());
  const minutes = pad(date.getMinutes());

  return `${year}-${month}-${day}T${hours}:${minutes}`;
};

const parseLocalDatetimeValue = (value: string): number | null => {
  if (value.trim() === '') {
    return null;
  }
  const date = new Date(value);
  const millis = date.getTime();
  if (Number.isNaN(millis)) {
    return null;
  }
  return Math.floor(millis / 1000);
};

const asFiniteNumber = (value: unknown): number | null => {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value;
  }
  return null;
};

const asFiniteString = (value: unknown): string | null => {
  if (typeof value === 'string' && value.trim() !== '') {
    return value;
  }
  return null;
};

const authModeFromValue = (value: unknown): AuthMode =>
  value === 'required' ? 'required' : 'off';

const buildAuthTransitionForm = (configJson: unknown): AuthTransitionForm => {
  const config = asRecord(configJson);
  const auth = asRecord(config?.auth);
  const mode = authModeFromValue(auth?.mode);
  const enforceAt = asFiniteNumber(auth?.enforce_at);

  return {
    mode,
    enforceMode: enforceAt === null ? 'immediate' : 'scheduled',
    enforceAtLocal: toLocalDatetimeValue(enforceAt),
    graceSeconds: String(Math.max(0, asFiniteNumber(auth?.grace_seconds) ?? 900)),
    wsAuthTimeoutSeconds: String(Math.max(1, asFiniteNumber(auth?.ws_auth_timeout_seconds) ?? 10))
  };
};

const computeAuthRuntimeStatus = (configJson: unknown): AuthRuntimeStatus => {
  const config = asRecord(configJson);
  const auth = asRecord(config?.auth);
  const mode = authModeFromValue(auth?.mode);
  const enforceAt = asFiniteNumber(auth?.enforce_at);
  const graceSeconds = Math.max(0, asFiniteNumber(auth?.grace_seconds) ?? 900);
  const disconnectAt = enforceAt === null ? null : enforceAt + graceSeconds;
  const now = Math.floor(Date.now() / 1000);

  if (mode === 'off') {
    return {
      phase: 'Auth off',
      status: 'inactive',
      enforceAt,
      disconnectAt,
      secondsUntilEnforce: null,
      secondsUntilDisconnect: null
    };
  }

  if (enforceAt === null) {
    return {
      phase: 'Required (immediate)',
      status: 'current',
      enforceAt,
      disconnectAt,
      secondsUntilEnforce: 0,
      secondsUntilDisconnect: null
    };
  }

  if (now < enforceAt) {
    return {
      phase: 'Scheduled',
      status: 'pending',
      enforceAt,
      disconnectAt,
      secondsUntilEnforce: enforceAt - now,
      secondsUntilDisconnect: disconnectAt === null ? null : disconnectAt - now
    };
  }

  if (disconnectAt !== null && now < disconnectAt) {
    return {
      phase: 'Grace period',
      status: 'pending',
      enforceAt,
      disconnectAt,
      secondsUntilEnforce: 0,
      secondsUntilDisconnect: disconnectAt - now
    };
  }

  return {
    phase: 'Required (enforced)',
    status: 'current',
    enforceAt,
    disconnectAt,
    secondsUntilEnforce: 0,
    secondsUntilDisconnect: 0
  };
};

const formatRemainingTime = (seconds: number | null): string => {
  if (seconds === null) {
    return '—';
  }
  const clamped = Math.max(0, Math.floor(seconds));
  const hours = Math.floor(clamped / 3600);
  const minutes = Math.floor((clamped % 3600) / 60);
  const secs = clamped % 60;
  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }
  if (minutes > 0) {
    return `${minutes}m ${secs}s`;
  }
  return `${secs}s`;
};

const formatMetricValue = (value: number | null): string => {
  if (value === null) {
    return '—';
  }
  return String(Math.round(value));
};

const parseRelayAuthMetrics = (service: ServiceInfo): RelayAuthMetrics | null => {
  if (service.service !== 'relay') {
    return null;
  }
  const details = asRecord(service.health?.details);
  const authTransition = asRecord(details?.auth_transition);
  if (!authTransition) {
    return null;
  }

  return {
    metricsStatus: asFiniteNumber(authTransition.metrics_status),
    wsConnections: asFiniteNumber(authTransition.ws_connections),
    wsUnauthenticatedConnections: asFiniteNumber(authTransition.ws_unauthenticated_connections),
    authRejectTotal: asFiniteNumber(authTransition.ingest_rejected_auth_total),
    authTimeoutDisconnectTotal: asFiniteNumber(authTransition.ws_auth_disconnect_timeout_total),
    authDeadlineDisconnectTotal: asFiniteNumber(authTransition.ws_auth_disconnect_deadline_total),
    metricsError: asFiniteString(authTransition.metrics_error)
  };
};

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

const AuthTransitionCard = ({ service }: { service: ServiceInfo }) => {
  const queryClient = useQueryClient();
  const [form, setForm] = useState<AuthTransitionForm>(() => buildAuthTransitionForm(service.config_json));
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    setForm(buildAuthTransitionForm(service.config_json));
  }, [service.version, service.config_json]);

  const mutation = useMutation({
    mutationFn: (payload: unknown) =>
      api.updateServiceConfig(service.service, payload, service.version),
    onSuccess: () => {
      setMessage('Auth transition settings saved.');
      queryClient.invalidateQueries({ queryKey: ['services'] });
    },
    onError: (err) => setMessage(errorToMessage(err))
  });

  const runtime = useMemo(() => computeAuthRuntimeStatus(service.config_json), [service.config_json]);
  const relayMetrics = useMemo(() => parseRelayAuthMetrics(service), [service]);

  const saveAuthConfig = () => {
    setMessage(null);
    const graceSeconds = Number(form.graceSeconds);
    if (!Number.isInteger(graceSeconds) || graceSeconds < 0) {
      setMessage('Grace seconds must be an integer (>= 0).');
      return;
    }

    const wsAuthTimeoutSeconds = Number(form.wsAuthTimeoutSeconds);
    if (!Number.isInteger(wsAuthTimeoutSeconds) || wsAuthTimeoutSeconds < 1) {
      setMessage('WS auth timeout seconds must be an integer (>= 1).');
      return;
    }

    let enforceAt: number | null = null;
    if (form.mode === 'required' && form.enforceMode === 'scheduled') {
      enforceAt = parseLocalDatetimeValue(form.enforceAtLocal);
      if (enforceAt === null) {
        setMessage('Enforce at must be a valid datetime.');
        return;
      }
    }

    const currentConfig = asRecord(service.config_json) ?? {};
    const currentAuth = asRecord(currentConfig.auth) ?? {};
    const payload = {
      ...currentConfig,
      auth: {
        ...currentAuth,
        mode: form.mode,
        enforce_at: enforceAt,
        grace_seconds: graceSeconds,
        ws_auth_timeout_seconds: wsAuthTimeoutSeconds
      }
    };

    mutation.mutate(payload);
  };

  const title = `${service.service} Auth Transition`;
  const subtitle =
    service.service === 'relay'
      ? 'NIP-42 AUTH required rollout with reservation/grace controls.'
      : 'Bootstrap auth rollout controls shared with User API bootstrap endpoints.';

  return (
    <Card>
      <CardHeader className="row">
        <div>
          <CardTitle>{title}</CardTitle>
          <p>{subtitle}</p>
        </div>
        <StatusBadge status={service.health?.status ?? 'unknown'} />
      </CardHeader>
      <CardContent>
        <div className="muted">
          Version {service.version} | Updated {formatTimestamp(service.updated_at)} by {service.updated_by}
        </div>

        <div className="grid">
          <div className="card sub-card">
            <h3>Auth Settings</h3>
            <div className="field">
              <Label htmlFor={`${service.service}-auth-mode`}>Auth mode</Label>
              <Select
                id={`${service.service}-auth-mode`}
                value={form.mode}
                onChange={(event) =>
                  setForm((prev) => ({
                    ...prev,
                    mode: authModeFromValue(event.target.value)
                  }))
                }
              >
                <option value="off">off</option>
                <option value="required">required</option>
              </Select>
            </div>

            <div className="field">
              <Label htmlFor={`${service.service}-enforce-mode`}>Enforce timing</Label>
              <Select
                id={`${service.service}-enforce-mode`}
                value={form.enforceMode}
                disabled={form.mode !== 'required'}
                onChange={(event) =>
                  setForm((prev) => ({
                    ...prev,
                    enforceMode: event.target.value === 'scheduled' ? 'scheduled' : 'immediate'
                  }))
                }
              >
                <option value="immediate">Immediate (enforce_at = null)</option>
                <option value="scheduled">Scheduled (use enforce_at)</option>
              </Select>
            </div>

            <div className="field">
              <Label htmlFor={`${service.service}-enforce-at`}>Enforce at</Label>
              <Input
                id={`${service.service}-enforce-at`}
                type="datetime-local"
                value={form.enforceAtLocal}
                disabled={form.mode !== 'required' || form.enforceMode !== 'scheduled'}
                onChange={(event) =>
                  setForm((prev) => ({
                    ...prev,
                    enforceAtLocal: event.target.value
                  }))
                }
              />
            </div>

            <div className="field">
              <Label htmlFor={`${service.service}-grace-seconds`}>Grace seconds</Label>
              <Input
                id={`${service.service}-grace-seconds`}
                type="number"
                min={0}
                step={1}
                value={form.graceSeconds}
                onChange={(event) =>
                  setForm((prev) => ({
                    ...prev,
                    graceSeconds: event.target.value
                  }))
                }
              />
            </div>

            <div className="field">
              <Label htmlFor={`${service.service}-ws-auth-timeout-seconds`}>
                WS auth timeout seconds
              </Label>
              <Input
                id={`${service.service}-ws-auth-timeout-seconds`}
                type="number"
                min={1}
                step={1}
                value={form.wsAuthTimeoutSeconds}
                onChange={(event) =>
                  setForm((prev) => ({
                    ...prev,
                    wsAuthTimeoutSeconds: event.target.value
                  }))
                }
              />
            </div>
          </div>

          <div className="card sub-card">
            <h3>Enforcement Status</h3>
            <div className="row">
              <div className="muted">Current phase</div>
              <StatusBadge status={runtime.status} label={runtime.phase} />
            </div>
            <table className="table">
              <tbody>
                <tr>
                  <td>enforce_at</td>
                  <td>{formatTimestamp(runtime.enforceAt)}</td>
                </tr>
                <tr>
                  <td>disconnect_unauthenticated_at</td>
                  <td>{formatTimestamp(runtime.disconnectAt)}</td>
                </tr>
                <tr>
                  <td>Time until enforce</td>
                  <td>{formatRemainingTime(runtime.secondsUntilEnforce)}</td>
                </tr>
                <tr>
                  <td>Time until unauth disconnect</td>
                  <td>{formatRemainingTime(runtime.secondsUntilDisconnect)}</td>
                </tr>
              </tbody>
            </table>

            {service.service === 'relay' && (
              <>
                <div className="divider" />
                <h3>Relay Runtime Signals</h3>
                {!relayMetrics && (
                  <Notice>Relay auth runtime metrics are unavailable. Check admin-api health poll.</Notice>
                )}
                {relayMetrics && (
                  <>
                    {relayMetrics.metricsError && <Notice tone="error">{relayMetrics.metricsError}</Notice>}
                    <table className="table">
                      <tbody>
                        <tr>
                          <td>Metrics status</td>
                          <td>{formatMetricValue(relayMetrics.metricsStatus)}</td>
                        </tr>
                        <tr>
                          <td>WS connections</td>
                          <td>{formatMetricValue(relayMetrics.wsConnections)}</td>
                        </tr>
                        <tr>
                          <td>Unauthenticated connections remaining</td>
                          <td>{formatMetricValue(relayMetrics.wsUnauthenticatedConnections)}</td>
                        </tr>
                        <tr>
                          <td>Auth-required rejects total</td>
                          <td>{formatMetricValue(relayMetrics.authRejectTotal)}</td>
                        </tr>
                        <tr>
                          <td>Auth timeout disconnects</td>
                          <td>{formatMetricValue(relayMetrics.authTimeoutDisconnectTotal)}</td>
                        </tr>
                        <tr>
                          <td>Auth deadline disconnects</td>
                          <td>{formatMetricValue(relayMetrics.authDeadlineDisconnectTotal)}</td>
                        </tr>
                      </tbody>
                    </table>
                  </>
                )}
              </>
            )}
          </div>
        </div>

        {message && <Notice tone={message.includes('saved') ? 'success' : 'error'}>{message}</Notice>}
        <Button onClick={saveAuthConfig} disabled={mutation.isPending}>
          {mutation.isPending ? 'Saving...' : 'Save auth transition'}
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

  const relayService = useMemo(
    () => (data ?? []).find((service) => service.service === 'relay') ?? null,
    [data]
  );
  const bootstrapService = useMemo(
    () => (data ?? []).find((service) => service.service === 'bootstrap') ?? null,
    [data]
  );
  const genericServices = useMemo(
    () => (data ?? []).filter((service) => !AUTH_FORM_TARGETS.has(service.service)),
    [data]
  );

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

      <div className="stack">
        {relayService ? <AuthTransitionCard service={relayService} /> : <Notice>Relay service is unavailable.</Notice>}
        {bootstrapService ? (
          <AuthTransitionCard service={bootstrapService} />
        ) : (
          <Notice>Bootstrap service is unavailable.</Notice>
        )}
      </div>

      <div className="grid">
        {genericServices.map((service) => (
          <ServiceCard key={service.service} service={service} />
        ))}
      </div>
    </>
  );
};
