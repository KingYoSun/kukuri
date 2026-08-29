import type { DomeHostingView, DomeTransitionAdmissionTicketV1 } from '@/lib/api';

const RETRY_DELAYS_MS = [0, 250, 1_000, 2_000, 5_000] as const;

export type DomeTransitionCommitRecoveryResult =
  | { status: 'committed' }
  | { status: 'rollback'; error: unknown }
  | { status: 'cancelled' };

type DomeTransitionCommitRecoveryOptions = {
  ticket: DomeTransitionAdmissionTicketV1;
  commit: () => Promise<void>;
  getHosting: () => Promise<DomeHostingView>;
  isCurrent: () => boolean;
  wait?: (delayMs: number) => Promise<void>;
};

function targetSessionWasReplaced(
  hosting: DomeHostingView,
  ticket: DomeTransitionAdmissionTicketV1
): boolean {
  if (hosting.state.kind === 'closed') return true;
  if (hosting.state.lease_epoch == null || hosting.state.session_id == null) return false;
  return hosting.state.lease_epoch !== ticket.target_lease_epoch
    || hosting.state.session_id !== ticket.target_session_id;
}

function isDefinitiveCommitRejection(error: unknown): boolean {
  const message = error instanceof Error ? error.message : String(error);
  return message.includes('DOME_TRANSITION_INVALID_TICKET')
    || message.includes('Dome transition admission ticket is invalid or expired')
    || message.includes('DOME_TRANSITION_STALE_TOPOLOGY')
    || message.includes('DOME_TRANSITION_STALE_SESSION')
    || message.includes('DOME_TRANSITION_OWNERS_BLOCKED')
    || message.includes('DOME_TRANSITION_VISITOR_BLOCKED')
    || message.includes('DOME_TRANSITION_ACCESS_DENIED');
}

export async function recoverDomeTransitionCommit({
  ticket,
  commit,
  getHosting,
  isCurrent,
  wait = (delayMs) => new Promise((resolve) => window.setTimeout(resolve, delayMs)),
}: DomeTransitionCommitRecoveryOptions): Promise<DomeTransitionCommitRecoveryResult> {
  let retryIndex = 0;
  while (isCurrent()) {
    try {
      await commit();
      return { status: 'committed' };
    } catch (error) {
      try {
        const hosting = await getHosting();
        if (targetSessionWasReplaced(hosting, ticket) || isDefinitiveCommitRejection(error)) {
          return { status: 'rollback', error };
        }
      } catch {
        // A hosting lookup failure cannot distinguish an applied commit from an unapplied one.
      }
      const delay = RETRY_DELAYS_MS[Math.min(retryIndex, RETRY_DELAYS_MS.length - 1)];
      retryIndex += 1;
      await wait(delay);
    }
  }
  return { status: 'cancelled' };
}
