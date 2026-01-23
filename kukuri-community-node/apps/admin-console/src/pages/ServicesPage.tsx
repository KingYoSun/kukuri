import { useEffect, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { api } from '../lib/api';
import { errorToMessage } from '../lib/errorHandler';
import { formatJson, formatTimestamp } from '../lib/format';
import type { ServiceInfo } from '../lib/types';
import { StatusBadge } from '../components/StatusBadge';

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
      mutation.mutate(parsed);
    } catch {
      setMessage('Config must be valid JSON.');
    }
  };

  return (
    <div className="card">
      <div className="row">
        <div>
          <h3>{service.service}</h3>
          <p>Version {service.version}</p>
        </div>
        <StatusBadge status={service.health?.status ?? 'unknown'} />
      </div>
      <div className="muted">
        Updated {formatTimestamp(service.updated_at)} by {service.updated_by}
      </div>
      <div className="field">
        <label htmlFor={`${service.service}-config`}>Config JSON</label>
        <textarea
          id={`${service.service}-config`}
          rows={10}
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
        />
      </div>
      {message && <div className="notice">{message}</div>}
      <button className="button" onClick={save} disabled={mutation.isPending}>
        {mutation.isPending ? 'Saving...' : 'Save config'}
      </button>
    </div>
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
      {isLoading && <div className="notice">Loading services...</div>}
      {error && <div className="notice">{errorToMessage(error)}</div>}
      <div className="grid">
        {(data ?? []).map((service) => (
          <ServiceCard key={service.service} service={service} />
        ))}
      </div>
    </>
  );
};
