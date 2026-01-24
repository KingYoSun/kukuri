import { useEffect, useMemo, useState } from 'react';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';

import { api } from '../lib/api';
import { errorToMessage } from '../lib/errorHandler';
import { formatTimestamp } from '../lib/format';
import type { TrustJob, TrustSchedule } from '../lib/types';
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

export const TrustPage = () => {
  const queryClient = useQueryClient();
  const [jobError, setJobError] = useState<string | null>(null);
  const [scheduleError, setScheduleError] = useState<string | null>(null);
  const [jobForm, setJobForm] = useState({
    job_type: jobTypeOptions[0].value,
    subject_pubkey: ''
  });
  const [scheduleEdits, setScheduleEdits] = useState<Record<string, ScheduleEdit>>({});

  const jobsQuery = useQuery<TrustJob[]>({
    queryKey: ['trust-jobs'],
    queryFn: () => api.trustJobs({ limit: 50 })
  });

  const schedulesQuery = useQuery<TrustSchedule[]>({
    queryKey: ['trust-schedules'],
    queryFn: api.trustSchedules
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
          <p>Recompute trust scores, issue attestations, and manage schedules.</p>
        </div>
        <button
          className="button"
          onClick={() => {
            void jobsQuery.refetch();
            void schedulesQuery.refetch();
          }}
        >
          Refresh
        </button>
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
    </>
  );
};
