import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { api } from '../lib/api';
import { asRecord, findServiceByName } from '../lib/config';
import { errorToMessage } from '../lib/errorHandler';
import { formatTimestamp } from '../lib/format';
import type { ServiceInfo, TrustJob, TrustSchedule, TrustTarget } from '../lib/types';
import { StatusBadge } from '../components/StatusBadge';

const jobTypeOptions = [
  { value: 'report_based', label: 'Report-based' },
  { value: 'communication_density', label: 'Communication density' }
];

const jobTypeLabel = (jobType: string) =>
  jobTypeOptions.find((option) => option.value === jobType)?.label ?? jobType;

type ScheduleEdit = {
  interval_seconds: string;
  is_enabled: boolean;
};

type TrustParameterForm = {
  enabled: boolean;
  reportWindowDays: string;
  reportWeight: string;
  labelWeight: string;
  reportScoreNormalization: string;
  communicationWindowDays: string;
  communicationScoreNormalization: string;
  interactionWeights: string;
  attestationExpSeconds: string;
};

const defaultTrustParameterForm = (): TrustParameterForm => ({
  enabled: false,
  reportWindowDays: '30',
  reportWeight: '1',
  labelWeight: '1',
  reportScoreNormalization: '10',
  communicationWindowDays: '30',
  communicationScoreNormalization: '20',
  interactionWeights: JSON.stringify({ '1': 1, '6': 0.5, '7': 0.3 }, null, 2),
  attestationExpSeconds: '86400'
});

const asFiniteNumber = (value: unknown, fallback: number): number => {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value;
  }
  return fallback;
};

const buildTrustParameterForm = (configJson: unknown): TrustParameterForm => {
  const defaults = defaultTrustParameterForm();
  const trustConfig = asRecord(configJson);
  const reportBased = asRecord(trustConfig?.report_based);
  const communicationDensity = asRecord(trustConfig?.communication_density);
  const attestation = asRecord(trustConfig?.attestation);

  const interactionWeights =
    communicationDensity?.interaction_weights &&
    typeof communicationDensity.interaction_weights === 'object' &&
    communicationDensity.interaction_weights !== null &&
    !Array.isArray(communicationDensity.interaction_weights)
      ? communicationDensity.interaction_weights
      : { '1': 1, '6': 0.5, '7': 0.3 };

  return {
    enabled: typeof trustConfig?.enabled === 'boolean' ? trustConfig.enabled : defaults.enabled,
    reportWindowDays: String(Math.max(1, asFiniteNumber(reportBased?.window_days, 30))),
    reportWeight: String(Math.max(0, asFiniteNumber(reportBased?.report_weight, 1))),
    labelWeight: String(Math.max(0, asFiniteNumber(reportBased?.label_weight, 1))),
    reportScoreNormalization: String(
      Math.max(1, asFiniteNumber(reportBased?.score_normalization, 10))
    ),
    communicationWindowDays: String(
      Math.max(1, asFiniteNumber(communicationDensity?.window_days, 30))
    ),
    communicationScoreNormalization: String(
      Math.max(1, asFiniteNumber(communicationDensity?.score_normalization, 20))
    ),
    interactionWeights: JSON.stringify(interactionWeights, null, 2),
    attestationExpSeconds: String(Math.max(60, asFiniteNumber(attestation?.exp_seconds, 86400)))
  };
};

export const TrustPage = () => {
  const queryClient = useQueryClient();
  const [jobError, setJobError] = useState<string | null>(null);
  const [scheduleError, setScheduleError] = useState<string | null>(null);
  const [parameterMessage, setParameterMessage] = useState<string | null>(null);
  const [parameterForm, setParameterForm] = useState<TrustParameterForm>(defaultTrustParameterForm());
  const [jobForm, setJobForm] = useState({
    job_type: jobTypeOptions[0].value,
    subject_pubkey: ''
  });
  const [targetInput, setTargetInput] = useState('');
  const [targetFilter, setTargetFilter] = useState('');
  const [scheduleEdits, setScheduleEdits] = useState<Record<string, ScheduleEdit>>({});

  const servicesQuery = useQuery<ServiceInfo[]>({
    queryKey: ['services'],
    queryFn: api.services
  });
  const trustService = useMemo(
    () => findServiceByName(servicesQuery.data, 'trust'),
    [servicesQuery.data]
  );

  const jobsQuery = useQuery<TrustJob[]>({
    queryKey: ['trust-jobs'],
    queryFn: () => api.trustJobs({ limit: 50 })
  });

  const schedulesQuery = useQuery<TrustSchedule[]>({
    queryKey: ['trust-schedules'],
    queryFn: api.trustSchedules
  });
  const targetsQuery = useQuery<TrustTarget[]>({
    queryKey: ['trust-targets', targetFilter],
    queryFn: () =>
      api.trustTargets({
        pubkey: targetFilter.trim() === '' ? undefined : targetFilter.trim(),
        limit: 100
      })
  });

  useEffect(() => {
    if (!schedulesQuery.data) {
      return;
    }
    setScheduleEdits((prev) => {
      const next = { ...prev };
      for (const schedule of schedulesQuery.data ?? []) {
        if (!next[schedule.job_type]) {
          next[schedule.job_type] = {
            interval_seconds: String(schedule.interval_seconds),
            is_enabled: schedule.is_enabled
          };
        }
      }
      return next;
    });
  }, [schedulesQuery.data]);

  useEffect(() => {
    if (trustService) {
      setParameterForm(buildTrustParameterForm(trustService.config_json));
    } else {
      setParameterForm(defaultTrustParameterForm());
    }
  }, [trustService?.version, trustService?.config_json]);

  const createJobMutation = useMutation({
    mutationFn: (payload: { job_type: string; subject_pubkey?: string | null }) =>
      api.createTrustJob(payload),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['trust-jobs'] });
      setJobError(null);
      setJobForm((prev) => ({ ...prev, subject_pubkey: '' }));
    },
    onError: (err) => setJobError(errorToMessage(err))
  });

  const updateScheduleMutation = useMutation({
    mutationFn: (payload: { jobType: string; interval_seconds: number; is_enabled: boolean }) =>
      api.updateTrustSchedule(payload.jobType, {
        interval_seconds: payload.interval_seconds,
        is_enabled: payload.is_enabled
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['trust-schedules'] });
      setScheduleError(null);
    },
    onError: (err) => setScheduleError(errorToMessage(err))
  });
  const saveParametersMutation = useMutation({
    mutationFn: (payload: unknown) =>
      api.updateServiceConfig('trust', payload, trustService?.version),
    onSuccess: () => {
      setParameterMessage('Trust parameters saved.');
      queryClient.invalidateQueries({ queryKey: ['services'] });
    },
    onError: (err) => setParameterMessage(errorToMessage(err))
  });

  const handleJobSubmit = () => {
    setJobError(null);
    const subject = jobForm.subject_pubkey.trim();
    createJobMutation.mutate({
      job_type: jobForm.job_type,
      subject_pubkey: subject === '' ? null : subject
    });
  };

  const handleScheduleSave = (jobType: string) => {
    setScheduleError(null);
    const edit = scheduleEdits[jobType];
    const interval = Number(edit?.interval_seconds);
    if (!edit || Number.isNaN(interval) || interval < 60) {
      setScheduleError('Interval must be a number (>= 60 seconds).');
      return;
    }
    updateScheduleMutation.mutate({
      jobType,
      interval_seconds: interval,
      is_enabled: edit.is_enabled
    });
  };

  const handleParameterSave = () => {
    setParameterMessage(null);
    if (!trustService) {
      setParameterMessage('Trust service config is unavailable.');
      return;
    }

    const reportWindowDays = Number(parameterForm.reportWindowDays);
    const reportWeight = Number(parameterForm.reportWeight);
    const labelWeight = Number(parameterForm.labelWeight);
    const reportScoreNormalization = Number(parameterForm.reportScoreNormalization);
    const communicationWindowDays = Number(parameterForm.communicationWindowDays);
    const communicationScoreNormalization = Number(parameterForm.communicationScoreNormalization);
    const attestationExpSeconds = Number(parameterForm.attestationExpSeconds);

    if (Number.isNaN(reportWindowDays) || reportWindowDays < 1) {
      setParameterMessage('Report window days must be 1 or greater.');
      return;
    }
    if (Number.isNaN(reportWeight) || reportWeight < 0) {
      setParameterMessage('Report weight must be 0 or greater.');
      return;
    }
    if (Number.isNaN(labelWeight) || labelWeight < 0) {
      setParameterMessage('Label weight must be 0 or greater.');
      return;
    }
    if (Number.isNaN(reportScoreNormalization) || reportScoreNormalization < 1) {
      setParameterMessage('Report score normalization must be 1 or greater.');
      return;
    }
    if (Number.isNaN(communicationWindowDays) || communicationWindowDays < 1) {
      setParameterMessage('Communication window days must be 1 or greater.');
      return;
    }
    if (Number.isNaN(communicationScoreNormalization) || communicationScoreNormalization < 1) {
      setParameterMessage('Communication score normalization must be 1 or greater.');
      return;
    }
    if (Number.isNaN(attestationExpSeconds) || attestationExpSeconds < 60) {
      setParameterMessage('Attestation exp seconds must be 60 or greater.');
      return;
    }

    let interactionWeights: Record<string, number>;
    try {
      const parsed = JSON.parse(parameterForm.interactionWeights);
      if (
        typeof parsed !== 'object' ||
        parsed === null ||
        Array.isArray(parsed)
      ) {
        setParameterMessage('Interaction weights must be a JSON object.');
        return;
      }

      interactionWeights = {};
      for (const [key, value] of Object.entries(parsed)) {
        const kind = Number(key);
        const weight = Number(value);
        if (!Number.isInteger(kind) || kind < 0 || Number.isNaN(weight) || weight <= 0) {
          setParameterMessage(
            'Interaction weights must use non-negative integer keys and positive values.'
          );
          return;
        }
        interactionWeights[String(kind)] = weight;
      }
    } catch (err) {
      setParameterMessage(errorToMessage(err));
      return;
    }

    const currentConfig = asRecord(trustService.config_json) ?? {};
    const reportBased = asRecord(currentConfig.report_based) ?? {};
    const communicationDensity = asRecord(currentConfig.communication_density) ?? {};
    const attestation = asRecord(currentConfig.attestation) ?? {};

    saveParametersMutation.mutate({
      ...currentConfig,
      enabled: parameterForm.enabled,
      report_based: {
        ...reportBased,
        window_days: Math.floor(reportWindowDays),
        report_weight: reportWeight,
        label_weight: labelWeight,
        score_normalization: reportScoreNormalization
      },
      communication_density: {
        ...communicationDensity,
        window_days: Math.floor(communicationWindowDays),
        score_normalization: communicationScoreNormalization,
        interaction_weights: interactionWeights
      },
      attestation: {
        ...attestation,
        exp_seconds: Math.floor(attestationExpSeconds)
      }
    });
  };

  const handleRunNow = (jobType: string) => {
    setJobError(null);
    createJobMutation.mutate({ job_type: jobType, subject_pubkey: null });
  };

  const summary = useMemo(() => {
    const jobs = jobsQuery.data ?? [];
    const pending = jobs.filter((job) => job.status === 'pending').length;
    const running = jobs.filter((job) => job.status === 'running').length;
    const failed = jobs.filter((job) => job.status === 'failed').length;
    return { total: jobs.length, pending, running, failed };
  }, [jobsQuery.data]);

  return (
    <>
      <div className="hero">
        <div>
          <h1>Trust</h1>
          <p>Tune trust parameters, search targets, and manage trust jobs/schedules.</p>
        </div>
        <button
          className="button"
          onClick={() => {
            void servicesQuery.refetch();
            void jobsQuery.refetch();
            void schedulesQuery.refetch();
            void targetsQuery.refetch();
          }}
        >
          Refresh
        </button>
      </div>

      <div className="card">
        <div className="row">
          <div>
            <h3>Trust Parameters</h3>
            <p>Configure report/communication windows and scoring weights.</p>
          </div>
          {trustService && <StatusBadge status={trustService.health?.status ?? 'unknown'} />}
        </div>
        {!trustService && <div className="notice">Trust service config is unavailable.</div>}
        {trustService && (
          <>
            <div className="muted">
              Version {trustService.version} | Updated {formatTimestamp(trustService.updated_at)} by{' '}
              {trustService.updated_by}
            </div>
            <div className="grid">
              <div className="card sub-card">
                <h3>Report-based</h3>
                <div className="field">
                  <label htmlFor="trust-enabled">Trust enabled</label>
                  <select
                    id="trust-enabled"
                    value={parameterForm.enabled ? 'true' : 'false'}
                    onChange={(event) =>
                      setParameterForm((prev) => ({
                        ...prev,
                        enabled: event.target.value === 'true'
                      }))
                    }
                  >
                    <option value="true">true</option>
                    <option value="false">false</option>
                  </select>
                </div>
                <div className="field">
                  <label htmlFor="trust-report-window">Report window days</label>
                  <input
                    id="trust-report-window"
                    type="number"
                    min={1}
                    value={parameterForm.reportWindowDays}
                    onChange={(event) =>
                      setParameterForm((prev) => ({
                        ...prev,
                        reportWindowDays: event.target.value
                      }))
                    }
                  />
                </div>
                <div className="field">
                  <label htmlFor="trust-report-weight">Report weight</label>
                  <input
                    id="trust-report-weight"
                    type="number"
                    min={0}
                    step="0.1"
                    value={parameterForm.reportWeight}
                    onChange={(event) =>
                      setParameterForm((prev) => ({
                        ...prev,
                        reportWeight: event.target.value
                      }))
                    }
                  />
                </div>
                <div className="field">
                  <label htmlFor="trust-label-weight">Label weight</label>
                  <input
                    id="trust-label-weight"
                    type="number"
                    min={0}
                    step="0.1"
                    value={parameterForm.labelWeight}
                    onChange={(event) =>
                      setParameterForm((prev) => ({
                        ...prev,
                        labelWeight: event.target.value
                      }))
                    }
                  />
                </div>
                <div className="field">
                  <label htmlFor="trust-report-normalization">Report score normalization</label>
                  <input
                    id="trust-report-normalization"
                    type="number"
                    min={1}
                    step="0.1"
                    value={parameterForm.reportScoreNormalization}
                    onChange={(event) =>
                      setParameterForm((prev) => ({
                        ...prev,
                        reportScoreNormalization: event.target.value
                      }))
                    }
                  />
                </div>
              </div>

              <div className="card sub-card">
                <h3>Communication-density</h3>
                <div className="field">
                  <label htmlFor="trust-communication-window">Communication window days</label>
                  <input
                    id="trust-communication-window"
                    type="number"
                    min={1}
                    value={parameterForm.communicationWindowDays}
                    onChange={(event) =>
                      setParameterForm((prev) => ({
                        ...prev,
                        communicationWindowDays: event.target.value
                      }))
                    }
                  />
                </div>
                <div className="field">
                  <label htmlFor="trust-communication-normalization">
                    Communication score normalization
                  </label>
                  <input
                    id="trust-communication-normalization"
                    type="number"
                    min={1}
                    step="0.1"
                    value={parameterForm.communicationScoreNormalization}
                    onChange={(event) =>
                      setParameterForm((prev) => ({
                        ...prev,
                        communicationScoreNormalization: event.target.value
                      }))
                    }
                  />
                </div>
                <div className="field">
                  <label htmlFor="trust-interaction-weights">Interaction weights (JSON)</label>
                  <textarea
                    id="trust-interaction-weights"
                    rows={7}
                    value={parameterForm.interactionWeights}
                    onChange={(event) =>
                      setParameterForm((prev) => ({
                        ...prev,
                        interactionWeights: event.target.value
                      }))
                    }
                  />
                </div>
                <div className="field">
                  <label htmlFor="trust-attestation-exp">Attestation exp seconds</label>
                  <input
                    id="trust-attestation-exp"
                    type="number"
                    min={60}
                    value={parameterForm.attestationExpSeconds}
                    onChange={(event) =>
                      setParameterForm((prev) => ({
                        ...prev,
                        attestationExpSeconds: event.target.value
                      }))
                    }
                  />
                </div>
              </div>
            </div>

            {parameterMessage && <div className="notice">{parameterMessage}</div>}
            <button
              className="button"
              onClick={handleParameterSave}
              disabled={saveParametersMutation.isPending}
            >
              {saveParametersMutation.isPending ? 'Saving...' : 'Save trust parameters'}
            </button>
          </>
        )}
      </div>

      <div className="grid">
        <div className="card">
          <h3>Run Job</h3>
          <div className="field">
            <label>Job type</label>
            <select
              value={jobForm.job_type}
              onChange={(event) =>
                setJobForm((prev) => ({ ...prev, job_type: event.target.value }))
              }
            >
              {jobTypeOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </div>
          <div className="field">
            <label>Subject pubkey (optional)</label>
            <input
              value={jobForm.subject_pubkey}
              onChange={(event) =>
                setJobForm((prev) => ({ ...prev, subject_pubkey: event.target.value }))
              }
              placeholder="64-char hex pubkey"
            />
          </div>
          {jobError && <div className="notice">{jobError}</div>}
          <button className="button" onClick={handleJobSubmit} disabled={createJobMutation.isPending}>
            {createJobMutation.isPending ? 'Enqueueing...' : 'Enqueue job'}
          </button>
        </div>

        <div className="card">
          <div className="row">
            <div>
              <h3>Schedules</h3>
              <p>Interval and enable/disable controls.</p>
            </div>
            <div className="muted">Next run uses server time.</div>
          </div>
          {scheduleError && <div className="notice">{scheduleError}</div>}
          <table className="table">
            <thead>
              <tr>
                <th>Job</th>
                <th>Interval (sec)</th>
                <th>Next run</th>
                <th>Enabled</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              {(schedulesQuery.data ?? []).map((schedule) => {
                const edit = scheduleEdits[schedule.job_type] ?? {
                  interval_seconds: String(schedule.interval_seconds),
                  is_enabled: schedule.is_enabled
                };
                return (
                  <tr key={schedule.job_type}>
                    <td>{jobTypeLabel(schedule.job_type)}</td>
                    <td>
                      <input
                        type="number"
                        min={60}
                        value={edit.interval_seconds}
                        onChange={(event) =>
                          setScheduleEdits((prev) => ({
                            ...prev,
                            [schedule.job_type]: {
                              ...edit,
                              interval_seconds: event.target.value
                            }
                          }))
                        }
                      />
                    </td>
                    <td>{formatTimestamp(schedule.next_run_at)}</td>
                    <td>
                      <input
                        type="checkbox"
                        checked={edit.is_enabled}
                        onChange={(event) =>
                          setScheduleEdits((prev) => ({
                            ...prev,
                            [schedule.job_type]: {
                              ...edit,
                              is_enabled: event.target.checked
                            }
                          }))
                        }
                      />
                    </td>
                    <td>
                      <div className="row">
                        <button
                          className="button secondary"
                          onClick={() => handleScheduleSave(schedule.job_type)}
                          disabled={updateScheduleMutation.isPending}
                        >
                          Save
                        </button>
                        <button
                          className="button"
                          onClick={() => handleRunNow(schedule.job_type)}
                          disabled={createJobMutation.isPending}
                        >
                          Run now
                        </button>
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>

      <div className="card">
        <div className="row">
          <div>
            <h3>Recent Jobs</h3>
            <p className="muted">
              Total {summary.total} | Pending {summary.pending} | Running {summary.running} | Failed{' '}
              {summary.failed}
            </p>
          </div>
        </div>
        {jobsQuery.isLoading && <div className="notice">Loading jobs...</div>}
        {jobsQuery.error && <div className="notice">{errorToMessage(jobsQuery.error)}</div>}
        <table className="table">
          <thead>
            <tr>
              <th>Job</th>
              <th>Subject</th>
              <th>Status</th>
              <th>Progress</th>
              <th>Requested</th>
              <th>Started</th>
              <th>Completed</th>
              <th>Error</th>
            </tr>
          </thead>
          <tbody>
            {(jobsQuery.data ?? []).map((job) => {
              const total = job.total_targets ?? 0;
              const progress =
                total > 0 ? `${job.processed_targets}/${total}` : String(job.processed_targets);
              return (
                <tr key={job.job_id}>
                  <td>{jobTypeLabel(job.job_type)}</td>
                  <td>{job.subject_pubkey ?? 'all'}</td>
                  <td>
                    <StatusBadge status={job.status} />
                  </td>
                  <td>{progress}</td>
                  <td>{formatTimestamp(job.requested_at)}</td>
                  <td>{formatTimestamp(job.started_at)}</td>
                  <td>{formatTimestamp(job.completed_at)}</td>
                  <td>{job.error_message ?? '—'}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      <div className="card">
        <div className="row">
          <div>
            <h3>Target Search</h3>
            <p>Search subjects with report/communication trust scores.</p>
          </div>
        </div>
        <div className="field">
          <label htmlFor="trust-target-filter">Pubkey filter (exact/prefix)</label>
          <input
            id="trust-target-filter"
            value={targetInput}
            onChange={(event) => setTargetInput(event.target.value)}
            placeholder="64-char hex pubkey or prefix"
          />
        </div>
        <div className="row">
          <button className="button" onClick={() => setTargetFilter(targetInput.trim())}>
            Search targets
          </button>
          <button className="button secondary" onClick={() => void targetsQuery.refetch()}>
            Reload
          </button>
        </div>
        {targetsQuery.isLoading && <div className="notice">Loading targets...</div>}
        {targetsQuery.error && <div className="notice">{errorToMessage(targetsQuery.error)}</div>}
        <table className="table">
          <thead>
            <tr>
              <th>Pubkey</th>
              <th>Report score</th>
              <th>Communication score</th>
              <th>Updated</th>
            </tr>
          </thead>
          <tbody>
            {(targetsQuery.data ?? []).map((target) => (
              <tr key={target.subject_pubkey}>
                <td>{target.subject_pubkey}</td>
                <td>
                  {target.report_score === null
                    ? '—'
                    : `${target.report_score.toFixed(3)} (${target.report_count ?? 0})`}
                </td>
                <td>
                  {target.communication_score === null
                    ? '—'
                    : `${target.communication_score.toFixed(3)} (${target.interaction_count ?? 0})`}
                </td>
                <td>{formatTimestamp(target.updated_at)}</td>
              </tr>
            ))}
            {(targetsQuery.data ?? []).length === 0 && (
              <tr>
                <td colSpan={4}>No trust targets found.</td>
              </tr>
            )}
          </tbody>
        </table>
      </div>
    </>
  );
};
