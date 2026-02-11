import { useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { StatusBadge } from '../components/StatusBadge';
import { api } from '../lib/api';
import { errorToMessage } from '../lib/errorHandler';
import { formatTimestamp } from '../lib/format';
import type {
  AccessControlInvite,
  AccessControlMembership,
  AuditLog,
  RevokeAccessControlResponse,
  RotateAccessControlResponse
} from '../lib/types';

const scopeOptions = [
  { value: 'invite', label: 'invite' },
  { value: 'friend', label: 'friend' },
  { value: 'friend_plus', label: 'friend_plus' }
];
const scopeFilterOptions = [{ value: '', label: 'all' }, ...scopeOptions];

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
  const [membershipForm, setMembershipForm] = useState({
    topic_id: '',
    scope: '',
    pubkey: '',
    status: 'active'
  });
  const [membershipFilters, setMembershipFilters] = useState({
    topic_id: '',
    scope: '',
    pubkey: '',
    status: 'active'
  });
  const [inviteIssueForm, setInviteIssueForm] = useState({
    topic_id: '',
    expiresInHours: '24',
    maxUses: '1',
    nonce: ''
  });
  const [inviteSearchForm, setInviteSearchForm] = useState({
    topic_id: '',
    status: 'active'
  });
  const [inviteFilters, setInviteFilters] = useState({
    topic_id: '',
    status: 'active'
  });
  const [inviteMessage, setInviteMessage] = useState<string | null>(null);
  const [inviteResult, setInviteResult] = useState<AccessControlInvite | null>(null);

  const auditQuery = useQuery<AuditLog[]>({
    queryKey: ['auditLogs', 'access-control'],
    queryFn: () => api.auditLogs({ limit: 200 })
  });
  const membershipsQuery = useQuery<AccessControlMembership[]>({
    queryKey: ['access-control-memberships', membershipFilters],
    queryFn: () =>
      api.accessControlMemberships({
        topic_id: membershipFilters.topic_id || undefined,
        scope: membershipFilters.scope || undefined,
        pubkey: membershipFilters.pubkey || undefined,
        status: membershipFilters.status || undefined,
        limit: 200
      })
  });
  const invitesQuery = useQuery<AccessControlInvite[]>({
    queryKey: ['access-control-invites', inviteFilters],
    queryFn: () =>
      api.accessControlInvites({
        topic_id: inviteFilters.topic_id || undefined,
        status: inviteFilters.status || undefined,
        limit: 200
      })
  });

  const rotateMutation = useMutation({
    mutationFn: (payload: { topic_id: string; scope: string }) => api.rotateAccessControl(payload),
    onSuccess: (result) => {
      setRotateResult(result);
      setRotateMessage('Epoch rotation completed.');
      queryClient.invalidateQueries({ queryKey: ['auditLogs'] });
      queryClient.invalidateQueries({ queryKey: ['access-control-memberships'] });
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
      queryClient.invalidateQueries({ queryKey: ['access-control-memberships'] });
    },
    onError: (error) => setRevokeMessage(errorToMessage(error))
  });
  const issueInviteMutation = useMutation({
    mutationFn: (payload: {
      topic_id: string;
      scope: string;
      expires_in_seconds?: number | null;
      max_uses?: number | null;
      nonce?: string | null;
    }) => api.issueAccessControlInvite(payload),
    onSuccess: (result) => {
      setInviteResult(result);
      setInviteMessage('Invite capability issued.');
      queryClient.invalidateQueries({ queryKey: ['auditLogs'] });
      queryClient.invalidateQueries({ queryKey: ['access-control-invites'] });
    },
    onError: (error) => setInviteMessage(errorToMessage(error))
  });
  const revokeInviteMutation = useMutation({
    mutationFn: (nonce: string) => api.revokeAccessControlInvite(nonce),
    onSuccess: (result) => {
      setInviteResult(result);
      setInviteMessage('Invite capability revoked.');
      queryClient.invalidateQueries({ queryKey: ['auditLogs'] });
      queryClient.invalidateQueries({ queryKey: ['access-control-invites'] });
    },
    onError: (error) => setInviteMessage(errorToMessage(error))
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

  const submitMembershipSearch = () => {
    setMembershipFilters({
      topic_id: membershipForm.topic_id.trim(),
      scope: membershipForm.scope,
      pubkey: membershipForm.pubkey.trim(),
      status: membershipForm.status
    });
  };

  const submitInviteSearch = () => {
    setInviteFilters({
      topic_id: inviteSearchForm.topic_id.trim(),
      status: inviteSearchForm.status
    });
  };

  const submitIssueInvite = () => {
    const topicId = inviteIssueForm.topic_id.trim();
    const hours = Number(inviteIssueForm.expiresInHours);
    const maxUses = Number(inviteIssueForm.maxUses);
    setInviteMessage(null);
    setInviteResult(null);
    if (topicId === '') {
      setInviteMessage('Topic ID is required.');
      return;
    }
    if (Number.isNaN(hours) || hours <= 0) {
      setInviteMessage('Expires in hours must be positive.');
      return;
    }
    if (Number.isNaN(maxUses) || maxUses <= 0) {
      setInviteMessage('Max uses must be positive.');
      return;
    }

    issueInviteMutation.mutate({
      topic_id: topicId,
      scope: 'invite',
      expires_in_seconds: Math.floor(hours * 3600),
      max_uses: Math.floor(maxUses),
      nonce:
        inviteIssueForm.nonce.trim() === '' ? null : inviteIssueForm.nonce.trim()
    });
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

  const submitInviteRevoke = (nonce: string) => {
    revokeInviteMutation.mutate(nonce);
  };

  return (
    <>
      <div className="hero">
        <div>
          <h1>Access Control</h1>
          <p>Rotate epochs and revoke memberships for private topic scopes.</p>
        </div>
        <button
          className="button"
          onClick={() => {
            void membershipsQuery.refetch();
            void invitesQuery.refetch();
            void auditQuery.refetch();
          }}
        >
          Refresh
        </button>
      </div>

      <div className="grid">
        <div className="card">
          <h3>Memberships</h3>
          <p>Search topic memberships by topic, scope, and pubkey.</p>
          <div className="field">
            <label htmlFor="membership-topic-id">Topic ID filter</label>
            <input
              id="membership-topic-id"
              value={membershipForm.topic_id}
              onChange={(event) =>
                setMembershipForm((prev) => ({ ...prev, topic_id: event.target.value }))
              }
              placeholder="kukuri:topic:example"
            />
          </div>
          <div className="field">
            <label htmlFor="membership-scope">Scope filter</label>
            <select
              id="membership-scope"
              value={membershipForm.scope}
              onChange={(event) =>
                setMembershipForm((prev) => ({ ...prev, scope: event.target.value }))
              }
            >
              {scopeFilterOptions.map((scope) => (
                <option key={scope.value || 'all'} value={scope.value}>
                  {scope.label}
                </option>
              ))}
            </select>
          </div>
          <div className="field">
            <label htmlFor="membership-pubkey">Pubkey filter</label>
            <input
              id="membership-pubkey"
              value={membershipForm.pubkey}
              onChange={(event) =>
                setMembershipForm((prev) => ({ ...prev, pubkey: event.target.value }))
              }
              placeholder="pubkey prefix or 64-char hex"
            />
          </div>
          <div className="field">
            <label htmlFor="membership-status">Status filter</label>
            <select
              id="membership-status"
              value={membershipForm.status}
              onChange={(event) =>
                setMembershipForm((prev) => ({ ...prev, status: event.target.value }))
              }
            >
              <option value="">all</option>
              <option value="active">active</option>
              <option value="revoked">revoked</option>
            </select>
          </div>
          <div className="row">
            <button className="button" onClick={submitMembershipSearch}>
              Search memberships
            </button>
            <button className="button secondary" onClick={() => void membershipsQuery.refetch()}>
              Reload
            </button>
          </div>
          {membershipsQuery.isLoading && <div className="notice">Loading memberships...</div>}
          {membershipsQuery.error && (
            <div className="notice">{errorToMessage(membershipsQuery.error)}</div>
          )}
          <table className="table">
            <thead>
              <tr>
                <th>Topic</th>
                <th>Scope</th>
                <th>Pubkey</th>
                <th>Status</th>
                <th>Joined</th>
                <th>Revoked</th>
              </tr>
            </thead>
            <tbody>
              {(membershipsQuery.data ?? []).map((membership) => (
                <tr
                  key={`${membership.topic_id}:${membership.scope}:${membership.pubkey}`}
                >
                  <td>{membership.topic_id}</td>
                  <td>{membership.scope}</td>
                  <td>{membership.pubkey}</td>
                  <td>{membership.status}</td>
                  <td>{formatTimestamp(membership.joined_at)}</td>
                  <td>
                    {membership.revoked_at
                      ? `${formatTimestamp(membership.revoked_at)} (${membership.revoked_reason ?? 'n/a'})`
                      : '-'}
                  </td>
                </tr>
              ))}
              {(membershipsQuery.data ?? []).length === 0 && (
                <tr>
                  <td colSpan={6}>No memberships found.</td>
                </tr>
              )}
            </tbody>
          </table>
        </div>

        <div className="card">
          <h3>Invite Capability</h3>
          <p>Issue, search, and revoke invite.capability events.</p>
          <div className="field">
            <label htmlFor="invite-topic-id">Invite topic ID</label>
            <input
              id="invite-topic-id"
              value={inviteIssueForm.topic_id}
              onChange={(event) =>
                setInviteIssueForm((prev) => ({ ...prev, topic_id: event.target.value }))
              }
              placeholder="kukuri:topic:example"
            />
          </div>
          <div className="field">
            <label htmlFor="invite-expires-hours">Expires in hours</label>
            <input
              id="invite-expires-hours"
              type="number"
              min={1}
              value={inviteIssueForm.expiresInHours}
              onChange={(event) =>
                setInviteIssueForm((prev) => ({ ...prev, expiresInHours: event.target.value }))
              }
            />
          </div>
          <div className="field">
            <label htmlFor="invite-max-uses">Max uses</label>
            <input
              id="invite-max-uses"
              type="number"
              min={1}
              value={inviteIssueForm.maxUses}
              onChange={(event) =>
                setInviteIssueForm((prev) => ({ ...prev, maxUses: event.target.value }))
              }
            />
          </div>
          <div className="field">
            <label htmlFor="invite-nonce">Nonce (optional)</label>
            <input
              id="invite-nonce"
              value={inviteIssueForm.nonce}
              onChange={(event) =>
                setInviteIssueForm((prev) => ({ ...prev, nonce: event.target.value }))
              }
              placeholder="invite-nonce"
            />
          </div>
          {inviteMessage && <div className="notice">{inviteMessage}</div>}
          {inviteResult && (
            <div className="card sub-card">
              <div className="row">
                <div>
                  <strong>{inviteResult.nonce}</strong>
                  <div className="muted">
                    {inviteResult.topic_id} | {inviteResult.scope}
                  </div>
                </div>
                <StatusBadge status={inviteResult.status} />
              </div>
              <div className="muted">
                Uses {inviteResult.used_count}/{inviteResult.max_uses} | Expires{' '}
                {formatTimestamp(inviteResult.expires_at)}
              </div>
            </div>
          )}
          <div className="row">
            <button
              className="button"
              onClick={submitIssueInvite}
              disabled={issueInviteMutation.isPending}
            >
              {issueInviteMutation.isPending ? 'Issuing...' : 'Issue invite'}
            </button>
          </div>

          <div className="field">
            <label htmlFor="invite-search-topic">Invite topic filter</label>
            <input
              id="invite-search-topic"
              value={inviteSearchForm.topic_id}
              onChange={(event) =>
                setInviteSearchForm((prev) => ({ ...prev, topic_id: event.target.value }))
              }
              placeholder="kukuri:topic:example"
            />
          </div>
          <div className="field">
            <label htmlFor="invite-search-status">Invite status filter</label>
            <select
              id="invite-search-status"
              value={inviteSearchForm.status}
              onChange={(event) =>
                setInviteSearchForm((prev) => ({ ...prev, status: event.target.value }))
              }
            >
              <option value="">all</option>
              <option value="active">active</option>
              <option value="revoked">revoked</option>
              <option value="expired">expired</option>
              <option value="exhausted">exhausted</option>
            </select>
          </div>
          <div className="row">
            <button className="button secondary" onClick={submitInviteSearch}>
              Search invites
            </button>
            <button className="button secondary" onClick={() => void invitesQuery.refetch()}>
              Reload invites
            </button>
          </div>
          {invitesQuery.isLoading && <div className="notice">Loading invites...</div>}
          {invitesQuery.error && <div className="notice">{errorToMessage(invitesQuery.error)}</div>}
          <table className="table">
            <thead>
              <tr>
                <th>Nonce</th>
                <th>Status</th>
                <th>Uses</th>
                <th>Expires</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              {(invitesQuery.data ?? []).map((invite) => (
                <tr key={invite.nonce}>
                  <td>{invite.nonce}</td>
                  <td>{invite.status}</td>
                  <td>
                    {invite.used_count}/{invite.max_uses}
                  </td>
                  <td>{formatTimestamp(invite.expires_at)}</td>
                  <td>
                    <button
                      className="button secondary"
                      onClick={() => submitInviteRevoke(invite.nonce)}
                      disabled={revokeInviteMutation.isPending || invite.status === 'revoked'}
                    >
                      Revoke invite
                    </button>
                  </td>
                </tr>
              ))}
              {(invitesQuery.data ?? []).length === 0 && (
                <tr>
                  <td colSpan={5}>No invites found.</td>
                </tr>
              )}
            </tbody>
          </table>
        </div>

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
