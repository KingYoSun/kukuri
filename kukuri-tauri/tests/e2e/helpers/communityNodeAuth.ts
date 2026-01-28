import { communityNodeAuthFlow } from './bridge';

export async function runCommunityNodeAuthFlow(baseUrl: string) {
  const trimmed = baseUrl.trim();
  if (!trimmed) {
    throw new Error('Community node base URL is required');
  }
  return await communityNodeAuthFlow(trimmed);
}
