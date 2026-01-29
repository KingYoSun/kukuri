# Community Node E2E artefact ?? 2026?01?29?

## ??
- ???? E2E ???/artefact ? `test-results/community-node-e2e` ? `tmp/logs/community-node-e2e` ????
- CI ? artefact ??? Runbook ??????

## ????
- `scripts/docker/run-desktop-e2e.sh` ? `SCENARIO=community-node-e2e` ???????????
- `scripts/test-docker.{ps1,sh}` ? community node E2E ????????
- `.github/workflows/test.yml` ? artefact ??? `test-results/community-node-e2e` / `tmp/logs/community-node-e2e` ????
- `docs/01_project/activeContext/build_e2e_test.md` ? community-node-e2e ?????? CI ????????
- `docs/01_project/activeContext/tasks/priority/community_nodes_roadmap.md` ????????????

## ??
- `gh act --bind --workflows .github/workflows/test.yml --job format-check --container-options "--user 0" -P ubuntu-latest=ghcr.io/catthehacker/ubuntu:full-latest --env NPM_CONFIG_PREFIX=/tmp/npm-global --artifact-server-path tmp/act-artifacts`
  - docker pull ???????????: `tmp/logs/gh_act_format-check_20260129-172929.log`
- `gh act --bind --workflows .github/workflows/test.yml --job native-test-linux --container-options "--user 0" -P ubuntu-latest=ghcr.io/catthehacker/ubuntu:full-latest --env NPM_CONFIG_PREFIX=/tmp/npm-global --env CARGO_TEST_THREADS=1 --env RUST_TEST_THREADS=1 --artifact-server-path tmp/act-artifacts`
  - ????????Cache pnpm dependencies ? tar ?????????? PowerShell ??????: `tmp/logs/gh_act_native-test-linux_20260129-174033.log`
- `gh act --bind --workflows .github/workflows/test.yml --job format-check --container-options "--user 0" -P ubuntu-latest=ghcr.io/catthehacker/ubuntu:full-latest --env NPM_CONFIG_PREFIX=/tmp/npm-global --artifact-server-path tmp/act-artifacts`
  - ?????: `tmp/logs/gh_act_format-check_20260129-181129.log`
- `gh act --bind --workflows .github/workflows/test.yml --job native-test-linux --container-options "--user 0" -P ubuntu-latest=ghcr.io/catthehacker/ubuntu:full-latest --env NPM_CONFIG_PREFIX=/tmp/npm-global --env CARGO_TEST_THREADS=1 --env RUST_TEST_THREADS=1 --artifact-server-path tmp/act-artifacts`
  - ????????Cache Rust dependencies ? tar ?????????? PowerShell ??????: `tmp/logs/gh_act_native-test-linux_20260129-181243.log`
- `gh act --bind --workflows .github/workflows/test.yml --job native-test-linux --container-options "--user 0" -P ubuntu-latest=ghcr.io/catthehacker/ubuntu:full-latest --env NPM_CONFIG_PREFIX=/tmp/npm-global --env CARGO_TEST_THREADS=1 --env RUST_TEST_THREADS=1 --artifact-server-path tmp/act-artifacts`
  - ????????Rust dependencies ? tar ?????????? PowerShell ??????: `tmp/logs/gh_act_native-test-linux_20260129-221344.log`
- `./scripts/test-docker.ps1 e2e-community-node`
  - WDIO ???? Spec Files 14/14 passed?100%????: `tmp/logs/community-node-e2e/20260129-132109.log`
  - ??????? `e2e-community-node exit code: -1` ????????: `tmp/logs/test_docker_e2e_community_node_20260129-222045.log`

## ??
- ????? `tmp/logs/community-node-e2e/20260129-132109.log`?WDIO ????????
- ?????????????exit code ? WDIO ???????????????
