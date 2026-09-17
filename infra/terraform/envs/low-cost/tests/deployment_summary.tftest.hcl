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

run "generic_node_does_not_require_starter_topics" {
  command = plan
  assert {
    condition     = !contains(keys(google_monitoring_alert_policy.community_node), "index_empty") && !contains(keys(google_monitoring_alert_policy.community_node), "index_topics_missing")
    error_message = "An empty expected topic set must not create index availability alerts."
  }
}

run "expected_public_topics_are_monitored" {
  command = plan
  variables {
    deploy_indexer_stack  = true
    index_expected_topics = ["kukuri:topic:general", "kukuri:topic:test", "kukuri:topic:dev"]
  }
  assert {
    condition     = google_monitoring_alert_policy.community_node["index_empty"].conditions[0].condition_threshold[0].threshold_value == 0.5
    error_message = "An empty or unreadable expected-topic index must alert."
  }
  assert {
    condition     = google_monitoring_alert_policy.community_node["index_topics_missing"].conditions[0].condition_threshold[0].comparison == "COMPARISON_LT"
    error_message = "Missing expected topics must alert."
  }
  assert {
    condition     = google_monitoring_metric_descriptor.community_node["body_fetch_failures_recent"].value_type == "DOUBLE" && !contains(keys(google_monitoring_alert_policy.community_node), "body_fetch_failures_recent")
    error_message = "Peer-dependent body failures are separately observable, not safety-provider paging."
  }
}

run "invalid_expected_topic_is_rejected" {
  command = plan
  variables {
    index_expected_topics = ["topic'; SELECT 1; --"]
  }
  expect_failures = [var.index_expected_topics]
}

run "readiness_timer_survives_startup_rerun" {
  command = plan
  variables {
    deploy_indexer_stack = true
    operator_config_path = "tests/fixtures/test-operator-config.yaml"
  }
  assert {
    condition     = strcontains(nonsensitive(module.vm.startup_script), "OnBootSec=2min\nOnActiveSec=2min\nOnUnitActiveSec=5min\n")
    error_message = "The regenerated readiness timer must schedule a run after a startup rerun past the boot window (#1097)."
  }
  assert {
    condition     = strcontains(nonsensitive(module.vm.startup_script), "systemctl daemon-reload\n") && strcontains(nonsensitive(module.vm.startup_script), "systemctl enable --now kukuri-readiness.timer\n")
    error_message = "The readiness timer must be re-enabled by the generated startup script."
  }
}

run "readiness_timer_absent_without_operator_config" {
  command = plan
  variables {
    deploy_indexer_stack = true
  }
  assert {
    condition     = !strcontains(nonsensitive(module.vm.startup_script), "systemctl enable --now kukuri-readiness.timer")
    error_message = "Readiness activation must not be scheduled without an operator config."
  }
}
