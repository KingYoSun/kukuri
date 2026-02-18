import { fireEvent, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { api } from '../lib/api';
import { TrustPage } from './TrustPage';
import { renderWithQueryClient } from '../test/renderWithQueryClient';

vi.mock('../lib/api', () => ({
  api: {
    services: vi.fn(),
    updateServiceConfig: vi.fn(),
    trustJobs: vi.fn(),
    trustSchedules: vi.fn(),
    createTrustJob: vi.fn(),
    updateTrustSchedule: vi.fn(),
    trustTargets: vi.fn()
  }
}));

describe('TrustPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.services).mockResolvedValue([
      {
        service: 'trust',
        version: 3,
        config_json: {
          enabled: true,
          report_based: {
            window_days: 30,
            report_weight: 1,
            label_weight: 1,
            score_normalization: 10
          },
          communication_density: {
            window_days: 30,
            score_normalization: 20,
            interaction_weights: { '1': 1, '6': 0.5, '7': 0.3 }
          },
          attestation: { exp_seconds: 86400 }
        },
        updated_at: 1738809600,
        updated_by: 'admin',
        health: { status: 'healthy', checked_at: 1738809600, details: null }
      }
    ]);
    vi.mocked(api.trustJobs).mockResolvedValue([
      {
        job_id: 'job-1',
        job_type: 'report_based',
        subject_pubkey: null,
        status: 'pending',
        processed_targets: 0,
        total_targets: 10,
        requested_at: 1738809600,
        requested_by: 'admin',
        started_at: null,
        completed_at: null,
        error_message: null
      }
    ]);
    vi.mocked(api.trustSchedules).mockResolvedValue([
      {
        job_type: 'report_based',
        interval_seconds: 3600,
        is_enabled: true,
        next_run_at: 1738813200,
        updated_at: 1738809600
      }
    ]);
    vi.mocked(api.trustTargets).mockResolvedValue([
      {
        subject_pubkey: 'd'.repeat(64),
        report_score: 0.8,
        report_count: 3,
        report_window_start: 1738723200,
        report_window_end: 1738809600,
        communication_score: 0.6,
        interaction_count: 7,
        peer_count: 2,
        communication_window_start: 1738723200,
        communication_window_end: 1738809600,
        updated_at: 1738809600
      }
    ]);
    vi.mocked(api.createTrustJob).mockResolvedValue({
      job_id: 'job-2',
      job_type: 'communication_density',
      subject_pubkey: 'c'.repeat(64),
      status: 'pending',
      processed_targets: 0,
      total_targets: 0,
      requested_at: 1738809601,
      requested_by: 'admin',
      started_at: null,
      completed_at: null,
      error_message: null
    });
    vi.mocked(api.updateTrustSchedule).mockResolvedValue({
      job_type: 'report_based',
      interval_seconds: 7200,
      is_enabled: false,
      next_run_at: 1738816800,
      updated_at: 1738810000
    });
    vi.mocked(api.updateServiceConfig).mockResolvedValue({
      service: 'trust',
      version: 4,
      config_json: { enabled: true },
      updated_at: 1738810000,
      updated_by: 'admin'
    });
  });

  it('主要操作を送信し、ジョブ/スケジュール表示を維持できる', async () => {
    renderWithQueryClient(<TrustPage />);
    const user = userEvent.setup();

    expect(await screen.findByRole('heading', { name: 'Trust' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Trust Parameters' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Run Job' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Schedules' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Recent Jobs' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Target Search' })).toBeInTheDocument();
    expect(screen.getByRole('columnheader', { name: 'Interval (sec)' })).toBeInTheDocument();
    expect(screen.getByRole('columnheader', { name: 'Progress' })).toBeInTheDocument();
    await screen.findByRole('button', { name: 'Save' });
    const recentJobsCard = screen.getByRole('heading', { name: 'Recent Jobs' }).closest('.card');
    expect(recentJobsCard).not.toBeNull();
    const summaryText = (recentJobsCard as HTMLElement).querySelector('.muted');
    expect(summaryText).not.toBeNull();
    expect(summaryText).toHaveTextContent('Total 1');
    expect(summaryText).toHaveTextContent('Pending 1');
    expect(summaryText).toHaveTextContent('Running 0');
    expect(summaryText).toHaveTextContent('Failed 0');

    await user.clear(screen.getByLabelText('Report window days'));
    await user.type(screen.getByLabelText('Report window days'), '45');
    await user.click(screen.getByRole('button', { name: 'Save trust parameters' }));
    await waitFor(() => {
      expect(api.updateServiceConfig).toHaveBeenCalledWith(
        'trust',
        expect.objectContaining({
          report_based: expect.objectContaining({
            window_days: 45
          })
        }),
        3
      );
    });

    const runJobCard = screen.getByRole('heading', { name: 'Run Job' }).closest('.card');
    expect(runJobCard).not.toBeNull();
    const runJobSelect = (runJobCard as HTMLElement).querySelector('select');
    const runJobInput = (runJobCard as HTMLElement).querySelector('input');
    expect(runJobSelect).not.toBeNull();
    expect(runJobInput).not.toBeNull();
    fireEvent.change(runJobSelect as HTMLElement, {
      target: { value: 'communication_density' }
    });
    fireEvent.change(runJobInput as HTMLElement, {
      target: { value: 'c'.repeat(64) }
    });
    await user.click(screen.getByRole('button', { name: 'Enqueue job' }));
    await waitFor(() => {
      expect(api.createTrustJob).toHaveBeenCalledWith({
        job_type: 'communication_density',
        subject_pubkey: 'c'.repeat(64)
      });
    });

    fireEvent.change(screen.getByDisplayValue('3600'), { target: { value: '7200' } });
    fireEvent.click(screen.getByRole('checkbox'));
    await user.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() => {
      expect(api.updateTrustSchedule).toHaveBeenCalledWith('report_based', {
        interval_seconds: 7200,
        is_enabled: false
      });
    });

    await user.click(screen.getByRole('button', { name: 'Run now' }));
    await waitFor(() => {
      expect(api.createTrustJob).toHaveBeenCalledWith({
        job_type: 'report_based',
        subject_pubkey: null
      });
    });

    await user.type(screen.getByLabelText('Pubkey filter (exact/prefix)'), 'd'.repeat(16));
    await user.click(screen.getByRole('button', { name: 'Search targets' }));
    await waitFor(() => {
      expect(api.trustTargets).toHaveBeenLastCalledWith({
        pubkey: 'd'.repeat(16),
        limit: 100
      });
    });

    await user.click(screen.getByRole('button', { name: 'Refresh' }));
    await waitFor(() => {
      expect(vi.mocked(api.services).mock.calls.length).toBeGreaterThan(1);
      expect(vi.mocked(api.trustJobs).mock.calls.length).toBeGreaterThan(1);
      expect(vi.mocked(api.trustSchedules).mock.calls.length).toBeGreaterThan(1);
      expect(vi.mocked(api.trustTargets).mock.calls.length).toBeGreaterThan(1);
    });
  });
});
