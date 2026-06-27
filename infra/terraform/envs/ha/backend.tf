# GCS backend (partial config)。
#   terraform init -backend-config=backend.hcl
terraform {
  backend "gcs" {}
}
