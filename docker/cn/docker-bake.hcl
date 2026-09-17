# #1122: 4 つの community node image を 1 回の build session で作る。
#
# builder stage は 4 package 分をまとめて compile し、4 target で共有する。各 image は
# OCI layout（directory）へ書き出すだけで、registry へは出さない。smoke が通ってから
# `docker buildx imagetools create` で GHCR へ push する（`kukuri-cn-images.yml`）。
# cn-indexer だけは smoke 用に docker archive も出す（docker exporter は attestation を
# 扱えないため、attestation を付ける production 用 target とは別 target にする）。

variable "TARGET_PACKAGES" {
  default = "kukuri-cn-user-api kukuri-cn-iroh-relay kukuri-cn-cli kukuri-cn-indexer"
}

variable "OUT_DIR" {
  default = "cn-images-out"
}

variable "SOURCE_URL" {
  default = ""
}

variable "REVISION" {
  default = ""
}

group "default" {
  targets = ["user-api", "iroh-relay", "cli", "indexer", "indexer-smoke"]
}

target "_common" {
  context    = "."
  dockerfile = "docker/cn/Dockerfile"
  args = {
    TARGET_PACKAGES = TARGET_PACKAGES
  }
}

function "image_labels" {
  params = [title]
  result = {
    "org.opencontainers.image.source"   = SOURCE_URL
    "org.opencontainers.image.revision" = REVISION
    "org.opencontainers.image.title"    = title
  }
}

target "user-api" {
  inherits = ["_common"]
  args     = { TARGET_BIN = "cn-user-api" }
  labels   = image_labels("kukuri-cn-user-api")
  attest   = ["type=provenance,mode=max"]
  output   = ["type=oci,dest=${OUT_DIR}/kukuri-cn-user-api,tar=false"]
}

target "iroh-relay" {
  inherits = ["_common"]
  args     = { TARGET_BIN = "cn-iroh-relay" }
  labels   = image_labels("kukuri-cn-iroh-relay")
  attest   = ["type=provenance,mode=max"]
  output   = ["type=oci,dest=${OUT_DIR}/kukuri-cn-iroh-relay,tar=false"]
}

target "cli" {
  inherits = ["_common"]
  args     = { TARGET_BIN = "cn-cli" }
  labels   = image_labels("kukuri-cn-cli")
  attest   = ["type=provenance,mode=max"]
  output   = ["type=oci,dest=${OUT_DIR}/kukuri-cn-cli,tar=false"]
}

target "indexer" {
  inherits = ["_common"]
  args     = { TARGET_BIN = "cn-indexer" }
  labels   = image_labels("kukuri-cn-indexer")
  attest   = ["type=provenance,mode=max"]
  output   = ["type=oci,dest=${OUT_DIR}/kukuri-cn-indexer,tar=false"]
}

# smoke 用。production target と同じ stage・同じ build args から作るため中身は同じで、
# attestation を持たない点だけが違う。push には使わない。
target "indexer-smoke" {
  inherits = ["_common"]
  args     = { TARGET_BIN = "cn-indexer" }
  tags     = ["cn-indexer-smoke:local"]
  output   = ["type=docker,dest=${OUT_DIR}/kukuri-cn-indexer-smoke.tar"]
}
