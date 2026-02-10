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
export type ReindexResponse = SuccessBody<'/v1/reindex', 'post'>;
export type AccessControlMembership = ArrayItem<
  SuccessBody<'/v1/admin/access-control/memberships', 'get'>
>;
export type RotateAccessControlResponse = SuccessBody<'/v1/admin/access-control/rotate', 'post'>;
export type RevokeAccessControlResponse = SuccessBody<'/v1/admin/access-control/revoke', 'post'>;

export type DashboardOutboxConsumer = {
  consumer: string;
  last_seq: number;
  backlog: number;
};

export type DashboardOutboxBacklog = {
  max_seq: number;
  total_backlog: number;
  max_backlog: number;
  threshold: number;
  alert: boolean;
  consumers: DashboardOutboxConsumer[];
};

export type DashboardRejectSurge = {
  source_status: string;
  source_error: string | null;
  current_total: number | null;
  previous_total: number | null;
  delta: number | null;
  per_minute: number | null;
  threshold_per_minute: number;
  alert: boolean;
};

export type DashboardDbPressure = {
  db_size_bytes: number;
  disk_soft_limit_bytes: number;
  disk_utilization: number;
  active_connections: number;
  max_connections: number;
  connection_utilization: number;
  lock_waiters: number;
  connection_threshold: number;
  lock_waiter_threshold: number;
  alert: boolean;
  alerts: string[];
};

export type DashboardSnapshot = {
  collected_at: number;
  outbox_backlog: DashboardOutboxBacklog;
  reject_surge: DashboardRejectSurge;
  db_pressure: DashboardDbPressure;
};
