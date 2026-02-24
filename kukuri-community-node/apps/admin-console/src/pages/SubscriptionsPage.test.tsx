import { fireEvent, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { api } from '../lib/api';
import { SubscriptionsPage } from './SubscriptionsPage';
import { renderWithQueryClient } from '../test/renderWithQueryClient';

vi.mock('../lib/api', () => ({
  api: {
    subscriptionRequests: vi.fn(),
    approveRequest: vi.fn(),
    rejectRequest: vi.fn(),
    plans: vi.fn(),
    createPlan: vi.fn(),
    updatePlan: vi.fn(),
    subscriptions: vi.fn(),
    updateSubscription: vi.fn(),
    usage: vi.fn()
  }
}));

describe('SubscriptionsPage', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(api.subscriptionRequests).mockResolvedValue([
      {
        request_id: 'req-1',
        topic_id: 'kukuri:topic:1',
        requester_pubkey: 'a'.repeat(64),
        requested_services: ['index', 'trust'],
        status: 'pending',
        created_at: 1738809600,
        reviewed_at: null
      }
    ]);
    vi.mocked(api.approveRequest).mockResolvedValue({ status: 'approved' });
    vi.mocked(api.rejectRequest).mockResolvedValue({ status: 'rejected' });
    vi.mocked(api.plans).mockResolvedValue([
      {
        plan_id: 'basic',
        name: 'Basic',
        is_active: true,
        limits: [{ metric: 'index.search_requests', window: 'day', limit: 100 }]
      }
    ]);
    vi.mocked(api.createPlan).mockResolvedValue({
      plan_id: 'pro',
      name: 'Pro',
      is_active: true,
      limits: [{ metric: 'index.search_requests', window: 'day', limit: 500 }]
    });
    vi.mocked(api.updatePlan).mockResolvedValue({
      plan_id: 'basic',
      name: 'Basic Plus',
      is_active: false,
      limits: [{ metric: 'index.search_requests', window: 'day', limit: 120 }]
    });
    vi.mocked(api.subscriptions).mockResolvedValue([
      {
        subscription_id: 'sub-1',
        subscriber_pubkey: 'b'.repeat(64),
        plan_id: 'basic',
        status: 'active',
        started_at: 1738809600,
        ended_at: null
      }
    ]);
    vi.mocked(api.updateSubscription).mockResolvedValue({
      status: 'paused'
    });
    vi.mocked(api.usage).mockResolvedValue([
      {
        metric: 'index.search_requests',
        day: '2026-02-01',
        count: 12
      }
    ]);
  });

  it('主要操作を実行し、購読/プラン/利用量セクション表示を維持できる', async () => {
    renderWithQueryClient(<SubscriptionsPage />);

    expect(await screen.findByRole('heading', { name: 'Subscriptions', level: 1 })).toBeInTheDocument();
    expect(await screen.findByRole('heading', { name: 'Subscription Requests' })).toBeInTheDocument();
    expect(screen.queryByRole('heading', { name: 'Node Subscriptions' })).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Plans' })).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Usage' })).toBeInTheDocument();
    expect(screen.getByRole('columnheader', { name: 'Subscriber' })).toBeInTheDocument();
    expect(screen.getByRole('columnheader', { name: 'Metric' })).toBeInTheDocument();

    const user = userEvent.setup();
    const topicLabels = await screen.findAllByText('kukuri:topic:1');
    const requestCard = topicLabels[0].closest('.card');
    expect(requestCard).not.toBeNull();
    const reviewInput = (requestCard as HTMLElement).querySelector('input');
    expect(reviewInput).not.toBeNull();
    fireEvent.change(reviewInput as HTMLElement, { target: { value: 'looks good' } });
    await user.click(within(requestCard as HTMLElement).getByRole('button', { name: 'Approve' }));
    await waitFor(() => {
      expect(api.approveRequest).toHaveBeenCalledWith('req-1', 'looks good');
    });

    await user.click(within(requestCard as HTMLElement).getByRole('button', { name: 'Reject' }));
    await waitFor(() => {
      expect(api.rejectRequest).toHaveBeenCalledWith('req-1', 'looks good');
    });

    const createPlanButton = screen.getByRole('button', { name: 'Create plan' });
    const planEditor = createPlanButton.closest('.card');
    expect(planEditor).not.toBeNull();
    const planInputs = (planEditor as HTMLElement).querySelectorAll('input');
    const planSelect = (planEditor as HTMLElement).querySelector('select');
    const planTextarea = (planEditor as HTMLElement).querySelector('textarea');
    expect(planInputs.length).toBeGreaterThanOrEqual(2);
    expect(planSelect).not.toBeNull();
    expect(planTextarea).not.toBeNull();
    fireEvent.change(planInputs[0], { target: { value: 'pro' } });
    fireEvent.change(planInputs[1], { target: { value: 'Pro' } });
    fireEvent.change(planTextarea as HTMLElement, {
      target: {
        value: '[{"metric":"index.search_requests","window":"day","limit":500}]'
      }
    });
    await user.click(createPlanButton);
    await waitFor(() => {
      expect(api.createPlan).toHaveBeenCalledWith({
        plan_id: 'pro',
        name: 'Pro',
        is_active: true,
        limits: [{ metric: 'index.search_requests', window: 'day', limit: 500 }]
      });
    });

    await user.click(screen.getByRole('button', { name: 'Edit' }));
    fireEvent.change(planInputs[1], { target: { value: 'Basic Plus' } });
    fireEvent.change(planSelect as HTMLElement, { target: { value: 'false' } });
    fireEvent.change(planTextarea as HTMLElement, {
      target: {
        value: '[{"metric":"index.search_requests","window":"day","limit":120}]'
      }
    });
    await user.click(screen.getByRole('button', { name: 'Update plan' }));
    await waitFor(() => {
      expect(api.updatePlan).toHaveBeenCalledWith('basic', {
        name: 'Basic Plus',
        is_active: false,
        limits: [{ metric: 'index.search_requests', window: 'day', limit: 120 }]
      });
    });

    const updateCard = screen.getByRole('heading', { name: 'Update Subscription' }).closest('.card');
    expect(updateCard).not.toBeNull();
    const updateInputs = (updateCard as HTMLElement).querySelectorAll('input');
    const updateSelect = (updateCard as HTMLElement).querySelector('select');
    expect(updateInputs.length).toBeGreaterThanOrEqual(2);
    expect(updateSelect).not.toBeNull();
    fireEvent.change(updateInputs[0], { target: { value: 'b'.repeat(64) } });
    fireEvent.change(updateInputs[1], { target: { value: 'basic' } });
    fireEvent.change(updateSelect as HTMLElement, { target: { value: 'paused' } });
    await user.click(within(updateCard as HTMLElement).getByRole('button', { name: 'Apply' }));
    await waitFor(() => {
      expect(api.updateSubscription).toHaveBeenCalledWith('b'.repeat(64), {
        plan_id: 'basic',
        status: 'paused'
      });
    });

    fireEvent.change(screen.getByPlaceholderText('npub or hex'), { target: { value: 'b' } });
    await waitFor(() => {
      expect(api.subscriptions).toHaveBeenLastCalledWith('b');
    });

    const usageCard = screen.getByRole('heading', { name: 'Usage' }).closest('.card');
    expect(usageCard).not.toBeNull();
    const usageInputs = (usageCard as HTMLElement).querySelectorAll('input');
    expect(usageInputs.length).toBeGreaterThanOrEqual(3);
    fireEvent.change(usageInputs[0], { target: { value: 'b'.repeat(64) } });
    fireEvent.change(usageInputs[1], { target: { value: 'index.search_requests' } });
    fireEvent.change(usageInputs[2], { target: { value: '7' } });
    await user.click(within(usageCard as HTMLElement).getByRole('button', { name: 'Fetch usage' }));
    await waitFor(() => {
      expect(api.usage).toHaveBeenCalledWith('b'.repeat(64), 'index.search_requests', 7);
    });
    expect(await screen.findByText('2026-02-01')).toBeInTheDocument();
  });
});
