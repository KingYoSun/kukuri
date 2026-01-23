import { useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { z } from 'zod';

import { api } from '../lib/api';
import { errorToMessage } from '../lib/errorHandler';
import { formatTimestamp } from '../lib/format';
import type { Policy } from '../lib/types';
import { StatusBadge } from '../components/StatusBadge';

const policySchema = z.object({
  policy_type: z.string().min(1, 'Policy type is required'),
  version: z.string().min(1, 'Version is required'),
  locale: z.string().min(1, 'Locale is required'),
  title: z.string().min(1, 'Title is required'),
  content_md: z.string().min(1, 'Content is required')
});

export const PoliciesPage = () => {
  const queryClient = useQueryClient();
  const [mode, setMode] = useState<'create' | 'edit'>('create');
  const [editingPolicyId, setEditingPolicyId] = useState<string | null>(null);
  const [form, setForm] = useState({
    policy_type: 'terms',
    version: '',
    locale: 'ja-JP',
    title: '',
    content_md: ''
  });
  const [policyError, setPolicyError] = useState<string | null>(null);

  const policiesQuery = useQuery<Policy[]>({
    queryKey: ['policies'],
    queryFn: api.policies
  });

  const saveMutation = useMutation({
    mutationFn: (payload: Policy) => {
      if (mode === 'edit' && editingPolicyId) {
        return api.updatePolicy(editingPolicyId, {
          title: payload.title,
          content_md: payload.content_md
        });
      }
      return api.createPolicy({
        policy_type: payload.policy_type,
        version: payload.version,
        locale: payload.locale,
        title: payload.title,
        content_md: payload.content_md
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['policies'] });
      setPolicyError(null);
      setMode('create');
      setEditingPolicyId(null);
    },
    onError: (err) => {
      setPolicyError(errorToMessage(err));
    }
  });

  const publishMutation = useMutation({
    mutationFn: (policyId: string) => api.publishPolicy(policyId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['policies'] })
  });

  const currentMutation = useMutation({
    mutationFn: (policyId: string) => api.makeCurrentPolicy(policyId),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['policies'] })
  });

  const submit = () => {
    setPolicyError(null);
    const parsed = policySchema.safeParse(form);
    if (!parsed.success) {
      setPolicyError(parsed.error.issues[0]?.message ?? 'Invalid policy data');
      return;
    }
    saveMutation.mutate(parsed.data);
  };

  const startEdit = (policy: Policy) => {
    setMode('edit');
    setEditingPolicyId(policy.policy_id);
    setForm({
      policy_type: policy.policy_type,
      version: policy.version,
      locale: policy.locale,
      title: policy.title,
      content_md: policy.content_md
    });
  };

  const reset = () => {
    setMode('create');
    setEditingPolicyId(null);
    setForm({
      policy_type: 'terms',
      version: '',
      locale: 'ja-JP',
      title: '',
      content_md: ''
    });
    setPolicyError(null);
  };

  return (
    <>
      <div className="hero">
        <div>
          <h1>Policies</h1>
          <p>Manage terms and privacy policy versions.</p>
        </div>
        {mode === 'edit' && (
          <button className="button secondary" onClick={reset}>
            Reset form
          </button>
        )}
      </div>
      <div className="grid">
        <div className="card">
          <h3>{mode === 'edit' ? 'Edit Policy' : 'Create Policy'}</h3>
          <div className="field">
            <label>Type</label>
            <select
              value={form.policy_type}
              onChange={(event) =>
                setForm((prev) => ({ ...prev, policy_type: event.target.value }))
              }
              disabled={mode === 'edit'}
            >
              <option value="terms">terms</option>
              <option value="privacy">privacy</option>
            </select>
          </div>
          <div className="field">
            <label>Version</label>
            <input
              value={form.version}
              onChange={(event) => setForm((prev) => ({ ...prev, version: event.target.value }))}
              disabled={mode === 'edit'}
            />
          </div>
          <div className="field">
            <label>Locale</label>
            <input
              value={form.locale}
              onChange={(event) => setForm((prev) => ({ ...prev, locale: event.target.value }))}
              disabled={mode === 'edit'}
            />
          </div>
          <div className="field">
            <label>Title</label>
            <input
              value={form.title}
              onChange={(event) => setForm((prev) => ({ ...prev, title: event.target.value }))}
            />
          </div>
          <div className="field">
            <label>Content (Markdown)</label>
            <textarea
              rows={10}
              value={form.content_md}
              onChange={(event) =>
                setForm((prev) => ({ ...prev, content_md: event.target.value }))
              }
            />
          </div>
          {policyError && <div className="notice">{policyError}</div>}
          <button className="button" onClick={submit} disabled={saveMutation.isPending}>
            {saveMutation.isPending ? 'Saving...' : mode === 'edit' ? 'Update' : 'Create'}
          </button>
        </div>
        <div className="card">
          <h3>Policy List</h3>
          {policiesQuery.isLoading && <div className="notice">Loading policies...</div>}
          {policiesQuery.error && (
            <div className="notice">{errorToMessage(policiesQuery.error)}</div>
          )}
          <div className="stack">
            {(policiesQuery.data ?? []).map((policy) => (
              <div key={policy.policy_id} className="card sub-card">
                <div className="row">
                  <div>
                    <strong>{policy.title}</strong>
                    <div className="muted">
                      {policy.policy_type}:{policy.version} ({policy.locale})
                    </div>
                  </div>
                  <StatusBadge
                    status={policy.is_current ? 'current' : policy.published_at ? 'active' : 'inactive'}
                    label={policy.is_current ? 'Current' : policy.published_at ? 'Published' : 'Draft'}
                  />
                </div>
                <div className="muted">
                  Published {formatTimestamp(policy.published_at ?? null)} | Effective{' '}
                  {formatTimestamp(policy.effective_at ?? null)}
                </div>
                <div className="row">
                  <button className="button secondary" onClick={() => startEdit(policy)}>
                    Edit
                  </button>
                  <button
                    className="button"
                    onClick={() => publishMutation.mutate(policy.policy_id)}
                    disabled={publishMutation.isPending}
                  >
                    Publish
                  </button>
                  <button
                    className="button secondary"
                    onClick={() => currentMutation.mutate(policy.policy_id)}
                    disabled={currentMutation.isPending}
                  >
                    Make current
                  </button>
                </div>
              </div>
            ))}
            {(policiesQuery.data ?? []).length === 0 && (
              <div className="notice">No policies created yet.</div>
            )}
          </div>
        </div>
      </div>
    </>
  );
};
