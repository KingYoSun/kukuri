import type {
  DesktopApi,
  DomeTransitionAdmissionRequestV1,
  DomeTransitionAdmissionTicketV1,
} from '../types';
import type {
  AbortDomeTransitionRequest,
  CommitDomeTransitionRequest,
  PrepareDomeTransitionRequest,
} from '../types.generated';
import { invokeDesktop } from '../invoke/desktop';
import { command } from '../invoke/dispatch';

export const domeTransitionApi: Pick<
  DesktopApi,
  'prepareDomeTransition' | 'commitDomeTransition' | 'abortDomeTransition'
> = {
  prepareDomeTransition: command('prepareDomeTransition', async (
    admissionRequest: DomeTransitionAdmissionRequestV1
  ) => invokeDesktop<DomeTransitionAdmissionTicketV1>('prepare_dome_transition', {
    request: { request: admissionRequest } satisfies PrepareDomeTransitionRequest,
  })),
  commitDomeTransition: command('commitDomeTransition', async (ticket, position, rotation) => {
    await invokeDesktop<void>('commit_dome_transition', {
      request: { ticket, position, rotation } satisfies CommitDomeTransitionRequest,
    });
  }),
  abortDomeTransition: command('abortDomeTransition', async (ticket) => {
    await invokeDesktop<void>('abort_dome_transition', {
      request: { ticket } satisfies AbortDomeTransitionRequest,
    });
  }),
};
