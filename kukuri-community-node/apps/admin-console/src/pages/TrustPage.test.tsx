import { fireEvent, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { api } from '../lib/api';
import { TrustPage } from './TrustPage';
import { renderWithQueryClient } from '../test/renderWithQueryClient';

vi.mock('../lib/api', () => ({
  api: {
    trustJobs: vi.fn(),
    trustSchedules: vi.fn(),
    createTrustJob: vi.fn(),
    updateTrustSchedule: vi.fn()
  }
}));

describe('TrustPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.trustJobs).mockResolvedValue([
      {
        job_id: 'job-1',
        job_type: 'report_based',
        subject_pubkey: null,
        status: 'pending',
        processed_targets: 0,
        total_targets: 10,
        requested_at: 1738809600,
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
    vi.mocked(api.createTrustJob).mockResolvedValue({
      job_id: 'job-2',
      status: 'pending'
    });
    vi.mocked(api.updateTrustSchedule).mockResolvedValue({
      job_type: 'report_based',
      interval_seconds: 7200,
      is_enabled: false,
      next_run_at: 1738816800,
      updated_at: 1738810000
    });
  });

  it('主要操作を送信し、ジョブ/スケジュール表示を維持できる', async () => {
    renderWithQueryClient(<TrustPage />);

    expect(await screen.findByRole('heading', { name: 'Trust' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Run Job' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Schedules' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Recent Jobs' })).toBeInTheDocument();
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

    const user = userEvent.setup();
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

    await user.click(screen.getByRole('button', { name: 'Refresh' }));
    await waitFor(() => {
      expect(vi.mocked(api.trustJobs).mock.calls.length).toBeGreaterThan(1);
      expect(vi.mocked(api.trustSchedules).mock.calls.length).toBeGreaterThan(1);
    });
  });
});
