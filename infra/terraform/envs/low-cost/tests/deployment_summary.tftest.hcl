mock_provider "google" {}

variables {
  project_id                  = "test-project"
  api_domain                  = "api.example.com"
  relay_domain                = "relay.example.com"
  acme_email                  = "ops@example.com"
  cn_user_api_image           = "example/user-api@sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
  cn_iroh_relay_image         = "example/relay@sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
  cn_cli_image                = "example/cli@sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
  jwt_secret_id               = "test-jwt-secret"
  postgres_password_secret_id = "test-postgres-secret"
  backup_enabled              = false
}

run "indexer_stack_enabled_summary" {
  command = plan

  variables {
    deploy_indexer_stack = true
  }

  assert {
    condition     = output.deployment_profile_summary.index_moderation_trust == "provisioned (cn-indexer + ArcadeDB + relation analysis)"
    error_message = "deployment summary must report the enabled indexer stack as provisioned"
  }

  assert {
    condition     = google_monitoring_metric_descriptor.community_node["media_fetch_unavailable_total"].display_name == "Community Node media fetch unavailable total"
    error_message = "media fetch unavailability must remain observable as a dedicated metric"
  }

  assert {
    condition     = google_monitoring_alert_policy.community_node["provider"].display_name == "Community Node external safety provider failure"
    error_message = "the provider alert must identify external safety provider failures"
  }

  assert {
    condition     = !contains(keys(google_monitoring_alert_policy.community_node), "media_fetch_unavailable")
    error_message = "peer-dependent media fetch unavailability must not create a paging alert"
  }
}

run "indexer_stack_disabled_summary" {
  command = plan

  variables {
    deploy_indexer_stack = false
  }

  assert {
    condition     = output.deployment_profile_summary.index_moderation_trust == "not provisioned"
    error_message = "deployment summary must report a disabled indexer stack as not provisioned"
  }
}
