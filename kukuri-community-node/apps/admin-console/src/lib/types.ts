import type { paths } from '../generated/admin-api';

type JsonBody<T> = T extends { content: { 'application/json': infer U } } ? U : never;
type ResponseBody<
  Path extends keyof paths,
  Method extends keyof paths[Path],
  Status extends number
> = paths[Path][Method] extends { responses: infer Responses }
  ? Status extends keyof Responses
    ? JsonBody<Responses[Status]>
    : never
  : never;
type SuccessBody<
  Path extends keyof paths,
  Method extends keyof paths[Path]
> = ResponseBody<Path, Method, 200> | ResponseBody<Path, Method, 201>;
type ArrayItem<T> = T extends Array<infer U> ? U : never;

export type AdminUser = SuccessBody<'/v1/admin/auth/me', 'get'>;
export type ServiceInfo = ArrayItem<SuccessBody<'/v1/admin/services', 'get'>>;
export type ServiceHealth = NonNullable<ServiceInfo['health']>;
export type Policy = ArrayItem<SuccessBody<'/v1/admin/policies', 'get'>>;
export type SubscriptionRequest = ArrayItem<
  SuccessBody<'/v1/admin/subscription-requests', 'get'>
>;
export type NodeSubscription = ArrayItem<SuccessBody<'/v1/admin/node-subscriptions', 'get'>>;
export type Plan = ArrayItem<SuccessBody<'/v1/admin/plans', 'get'>>;
export type PlanLimit = ArrayItem<Plan['limits']>;
export type SubscriptionRow = ArrayItem<SuccessBody<'/v1/admin/subscriptions', 'get'>>;
export type UsageRow = ArrayItem<SuccessBody<'/v1/admin/usage', 'get'>>;
export type AuditLog = ArrayItem<SuccessBody<'/v1/admin/audit-logs', 'get'>>;
export type ModerationRule = ArrayItem<SuccessBody<'/v1/admin/moderation/rules', 'get'>>;
export type ModerationReport = ArrayItem<SuccessBody<'/v1/admin/moderation/reports', 'get'>>;
export type ModerationLabel = ArrayItem<SuccessBody<'/v1/admin/moderation/labels', 'get'>>;
export type TrustJob = ArrayItem<SuccessBody<'/v1/admin/trust/jobs', 'get'>>;
export type TrustSchedule = ArrayItem<SuccessBody<'/v1/admin/trust/schedules', 'get'>>;
