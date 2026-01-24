export type AdminUser = {
  admin_user_id: string;
  username: string;
};

export type ServiceHealth = {
  status: string;
  checked_at: number;
  details?: unknown | null;
};

export type ServiceInfo = {
  service: string;
  version: number;
  config_json: unknown;
  updated_at: number;
  updated_by: string;
  health?: ServiceHealth | null;
};

export type Policy = {
  policy_id: string;
  policy_type: string;
  version: string;
  locale: string;
  title: string;
  content_md: string;
  content_hash: string;
  published_at?: number | null;
  effective_at?: number | null;
  is_current: boolean;
};

export type SubscriptionRequest = {
  request_id: string;
  requester_pubkey: string;
  topic_id: string;
  requested_services: unknown;
  status: string;
  review_note?: string | null;
  created_at: number;
  reviewed_at?: number | null;
};

export type NodeSubscription = {
  topic_id: string;
  enabled: boolean;
  ref_count: number;
  updated_at: number;
};

export type PlanLimit = {
  metric: string;
  window: string;
  limit: number;
};

export type Plan = {
  plan_id: string;
  name: string;
  is_active: boolean;
  limits: PlanLimit[];
};

export type SubscriptionRow = {
  subscription_id: string;
  subscriber_pubkey: string;
  plan_id: string;
  status: string;
  started_at: number;
  ended_at?: number | null;
};

export type UsageRow = {
  metric: string;
  day: string;
  count: number;
};

export type AuditLog = {
  audit_id: number;
  actor_admin_user_id: string;
  action: string;
  target: string;
  diff_json?: unknown | null;
  request_id?: string | null;
  created_at: number;
};

export type ModerationRule = {
  rule_id: string;
  name: string;
  description?: string | null;
  is_enabled: boolean;
  priority: number;
  conditions: unknown;
  action: unknown;
  created_at: number;
  updated_at: number;
  updated_by: string;
};

export type ModerationReport = {
  report_id: string;
  reporter_pubkey: string;
  target: string;
  reason: string;
  created_at: number;
};

export type ModerationLabel = {
  label_id: string;
  target: string;
  topic_id?: string | null;
  label: string;
  confidence?: number | null;
  policy_url: string;
  policy_ref: string;
  exp: number;
  issuer_pubkey: string;
  rule_id?: string | null;
  source: string;
  issued_at: number;
};
