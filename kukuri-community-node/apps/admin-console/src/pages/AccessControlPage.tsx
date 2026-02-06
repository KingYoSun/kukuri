import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { StatusBadge } from '../components/StatusBadge';
import { api } from '../lib/api';
import { errorToMessage } from '../lib/errorHandler';
import { formatTimestamp } from '../lib/format';
import type { AuditLog, RevokeAccessControlResponse, RotateAccessControlResponse } from '../lib/types';

const scopeOptions = [
  { value: 'invite', label: 'invite' },
  { value: 'friend', label: 'friend' },
  { value: 'friend_plus', label: 'friend_plus' }
];

const pubkeyPattern = /^[0-9a-f]{64}$/i;

export const AccessControlPage = () => {
  const queryClient = useQueryClient();
  const [rotateForm, setRotateForm] = useState({ topic_id: '', scope: scopeOptions[0].value });
  const [revokeForm, setRevokeForm] = useState({
    topic_id: '',
    scope: scopeOptions[0].value,
    pubkey: '',
    reason: ''
  });
  const [rotateMessage, setRotateMessage] = useState<string | null>(null);
  const [revokeMessage, setRevokeMessage] = useState<string | null>(null);
  const [rotateResult, setRotateResult] = useState<RotateAccessControlResponse | null>(null);
  const [revokeResult, setRevokeResult] = useState<RevokeAccessControlResponse | null>(null);

  const auditQuery = useQuery<AuditLog[]>({
    queryKey: ['auditLogs', 'access-control'],
    queryFn: () => api.auditLogs({ limit: 200 })
  });

  const rotateMutation = useMutation({
    mutationFn: (payload: { topic_id: string; scope: string }) => api.rotateAccessControl(payload),
    onSuccess: (result) => {
      setRotateResult(result);
      setRotateMessage('Epoch rotation completed.');
      queryClient.invalidateQueries({ queryKey: ['auditLogs'] });
    },
    onError: (error) => setRotateMessage(errorToMessage(error))
  });

  const revokeMutation = useMutation({
    mutationFn: (payload: { topic_id: string; scope: string; pubkey: string; reason?: string | null }) =>
      api.revokeAccessControl(payload),
    onSuccess: (result) => {
      setRevokeResult(result);
      setRevokeMessage('Membership revoked and epoch rotated.');
      queryClient.invalidateQueries({ queryKey: ['auditLogs'] });
    },
    onError: (error) => setRevokeMessage(errorToMessage(error))
  });

  const accessControlAudits = useMemo(
    () => (auditQuery.data ?? []).filter((log) => log.action.startsWith('access_control.')),
    [auditQuery.data]
  );

  const submitRotate = () => {
    const topicId = rotateForm.topic_id.trim();
    setRotateMessage(null);
    setRotateResult(null);
    if (topicId === '') {
      setRotateMessage('Topic ID is required.');
      return;
    }
    rotateMutation.mutate({ topic_id: topicId, scope: rotateForm.scope });
  };

  const submitRevoke = () => {
    const topicId = revokeForm.topic_id.trim();
    const pubkey = revokeForm.pubkey.trim();
    setRevokeMessage(null);
    setRevokeResult(null);
    if (topicId === '') {
      setRevokeMessage('Topic ID is required.');
      return;
    }
    if (!pubkeyPattern.test(pubkey)) {
      setRevokeMessage('Pubkey must be a 64-char hex string.');
      return;
    }

    revokeMutation.mutate({
      topic_id: topicId,
      scope: revokeForm.scope,
      pubkey,
      reason: revokeForm.reason.trim() === '' ? null : revokeForm.reason.trim()
    });
  };

  return (
    <>
      <div className="hero">
        <div>
          <h1>Access Control</h1>
          <p>Rotate epochs and revoke memberships for private topic scopes.</p>
        </div>
        <button className="button" onClick={() => void auditQuery.refetch()}>
          Refresh
        </button>
      </div>

      <div className="grid">
        <div className="card">
          <h3>Rotate Epoch</h3>
          <p>Re-issue group keys for active members in the selected topic/scope.</p>
          <div className="field">
            <label htmlFor="rotate-topic-id">Topic ID</label>
            <input
              id="rotate-topic-id"
              value={rotateForm.topic_id}
              onChange={(event) =>
                setRotateForm((prev) => ({ ...prev, topic_id: event.target.value }))
              }
              placeholder="kukuri:topic:example"
            />
          </div>
          <div className="field">
            <label htmlFor="rotate-scope">Scope</label>
            <select
              id="rotate-scope"
              value={rotateForm.scope}
              onChange={(event) =>
                setRotateForm((prev) => ({ ...prev, scope: event.target.value }))
              }
            >
              {scopeOptions.map((scope) => (
                <option key={scope.value} value={scope.value}>
                  {scope.label}
                </option>
              ))}
            </select>
          </div>
          {rotateMessage && <div className="notice">{rotateMessage}</div>}
          {rotateResult && (
            <div className="card sub-card">
              <div className="row">
                <div>
                  <strong>{rotateResult.topic_id}</strong>
                  <div className="muted">Scope {rotateResult.scope}</div>
                </div>
                <StatusBadge status="active" label="Rotated" />
              </div>
              <div className="muted">
                Epoch {rotateResult.previous_epoch} → {rotateResult.new_epoch} / recipients{' '}
                {rotateResult.recipients}
              </div>
            </div>
          )}
          <button className="button" onClick={submitRotate} disabled={rotateMutation.isPending}>
            {rotateMutation.isPending ? 'Rotating...' : 'Rotate epoch'}
          </button>
        </div>

        <div className="card">
          <h3>Revoke Membership</h3>
          <p>Remove a member and rotate epoch in one operation.</p>
          <div className="field">
            <label htmlFor="revoke-topic-id">Topic ID</label>
            <input
              id="revoke-topic-id"
              value={revokeForm.topic_id}
              onChange={(event) =>
                setRevokeForm((prev) => ({ ...prev, topic_id: event.target.value }))
              }
              placeholder="kukuri:topic:example"
            />
          </div>
          <div className="field">
            <label htmlFor="revoke-scope">Scope</label>
            <select
              id="revoke-scope"
              value={revokeForm.scope}
              onChange={(event) =>
                setRevokeForm((prev) => ({ ...prev, scope: event.target.value }))
              }
            >
              {scopeOptions.map((scope) => (
                <option key={scope.value} value={scope.value}>
                  {scope.label}
                </option>
              ))}
            </select>
          </div>
          <div className="field">
            <label htmlFor="revoke-pubkey">Pubkey</label>
            <input
              id="revoke-pubkey"
              value={revokeForm.pubkey}
              onChange={(event) =>
                setRevokeForm((prev) => ({ ...prev, pubkey: event.target.value }))
              }
              placeholder="64-char hex pubkey"
            />
          </div>
          <div className="field">
            <label htmlFor="revoke-reason">Reason (optional)</label>
            <input
              id="revoke-reason"
              value={revokeForm.reason}
              onChange={(event) =>
                setRevokeForm((prev) => ({ ...prev, reason: event.target.value }))
              }
              placeholder="policy violation"
            />
          </div>
          {revokeMessage && <div className="notice">{revokeMessage}</div>}
          {revokeResult && (
            <div className="card sub-card">
              <div className="row">
                <div>
                  <strong>{revokeResult.revoked_pubkey}</strong>
                  <div className="muted">
                    {revokeResult.topic_id} / {revokeResult.scope}
                  </div>
                </div>
                <StatusBadge status="active" label="Revoked" />
              </div>
              <div className="muted">
                Epoch {revokeResult.previous_epoch} → {revokeResult.new_epoch} / recipients{' '}
                {revokeResult.recipients}
              </div>
            </div>
          )}
          <button className="button" onClick={submitRevoke} disabled={revokeMutation.isPending}>
            {revokeMutation.isPending ? 'Revoking...' : 'Revoke + rotate'}
          </button>
        </div>
      </div>

      <div className="card">
        <h3>Recent Access Control Audits</h3>
        {auditQuery.isLoading && <div className="notice">Loading audit logs...</div>}
        {auditQuery.error && <div className="notice">{errorToMessage(auditQuery.error)}</div>}
        <table className="table">
          <thead>
            <tr>
              <th>Time</th>
              <th>Action</th>
              <th>Target</th>
              <th>Actor</th>
            </tr>
          </thead>
          <tbody>
            {accessControlAudits.map((log) => (
              <tr key={log.audit_id}>
                <td>{formatTimestamp(log.created_at)}</td>
                <td>{log.action}</td>
                <td>{log.target}</td>
                <td>{log.actor_admin_user_id}</td>
              </tr>
            ))}
            {accessControlAudits.length === 0 && (
              <tr>
                <td colSpan={4}>No access-control audit logs found.</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </>
  );
};
