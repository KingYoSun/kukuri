# GCS backend (partial config)。
#
# 共有 / CI 運用では backend.hcl に bucket/prefix を渡して初期化する:
#   terraform init -backend-config=backend.hcl
#
# low-cost の単独 operator は backend ブロックをコメントアウトしたまま
# local backend で開始してもよい（plan の state backend 方針）。
terraform {
  backend "gcs" {}
}
