$ErrorActionPreference = "Stop"

$scriptPath = Join-Path $PSScriptRoot "update-changelog.ps1"
$workDir = Join-Path ([System.IO.Path]::GetTempPath()) ("kukuri-changelog-test-" + [System.Guid]::NewGuid())
$repoDir = Join-Path $workDir "repo"
$changelogPath = Join-Path $repoDir "CHANGELOG.md"
$sectionPath = Join-Path $workDir "CHANGELOG_SECTION.md"

function Invoke-GitOrThrow {
  param([string[]]$Arguments)
  & git @Arguments | Out-Null
  if ($LASTEXITCODE -ne 0) {
    throw "git $($Arguments -join ' ') failed with exit code $LASTEXITCODE"
  }
}

function New-Commit {
  param([string]$Message)
  Set-Content -LiteralPath (Join-Path $repoDir "log.txt") -Value $Message -Encoding UTF8
  Invoke-GitOrThrow @("add", "log.txt")
  Invoke-GitOrThrow @("-c", "commit.gpgsign=false", "commit", "-m", $Message)
}

try {
  New-Item -ItemType Directory -Force -Path $repoDir | Out-Null
  Push-Location $repoDir
  try {
    Invoke-GitOrThrow @("init", "--quiet")
    Invoke-GitOrThrow @("config", "user.email", "test@example.com")
    Invoke-GitOrThrow @("config", "user.name", "Test User")

    @"
# Changelog

## [Unreleased]
"@ | Set-Content -LiteralPath $changelogPath -Encoding UTF8

    # Commit before the previous tag: must NOT appear in the generated section.
    New-Commit "chore: scaffold repository (#100)"
    Invoke-GitOrThrow @("tag", "v0.1.0-preview.1")

    # Commits that belong to the release under test.
    New-Commit "feat: topic一覧にsearch/filter/sort機能を追加 (#340)"
    New-Commit "fix: 接続エラー復帰後の表示を修正 (#341)"
    New-Commit "refactor: tokenize elevation (#342)"
    New-Commit "feat: improve reply/thread UI (#307) (#337)"
    New-Commit "no conventional prefix change (#350)"
    Invoke-GitOrThrow @("tag", "v0.1.1-preview.1")

    & $scriptPath `
      -Tag "v0.1.1-preview.1" `
      -Repository "KingYoSun/kukuri" `
      -PreviousTag "v0.1.0-preview.1" `
      -ChangelogPath $changelogPath `
      -SectionOutputPath $sectionPath `
      -Date "2026-06-15" | Out-Null

    $section = Get-Content -LiteralPath $sectionPath -Raw -Encoding UTF8

    if ($section -notmatch '## \[v0\.1\.1-preview\.1\] - 2026-06-15') {
      throw "Section heading with tag and date missing"
    }
    foreach ($group in @("### Features", "### Fixes", "### Other")) {
      if ($section -notmatch [regex]::Escape($group)) {
        throw "Missing group heading: $group"
      }
    }
    if ($section -notmatch '\[#340\]\(https://github\.com/KingYoSun/kukuri/pull/340\)') {
      throw "PR #340 link missing or malformed"
    }
    if ($section -notmatch '\[#307\]\(https://github\.com/KingYoSun/kukuri/pull/307\)' -or
        $section -notmatch '\[#337\]\(https://github\.com/KingYoSun/kukuri/pull/337\)') {
      throw "Nested PR links (#307, #337) not both rendered"
    }
    if ($section -match '#100') {
      throw "Commit before the previous tag leaked into the section"
    }
    if ($section -match '\(#340\)') {
      throw "Raw (#340) reference was not stripped from the description text"
    }

    # Idempotency: re-running must not duplicate the section.
    & $scriptPath `
      -Tag "v0.1.1-preview.1" `
      -Repository "KingYoSun/kukuri" `
      -PreviousTag "v0.1.0-preview.1" `
      -ChangelogPath $changelogPath `
      -SectionOutputPath $sectionPath `
      -Date "2026-06-15" | Out-Null

    $changelog = Get-Content -LiteralPath $changelogPath -Raw -Encoding UTF8
    $headingCount = ([regex]::Matches($changelog, '## \[v0\.1\.1-preview\.1\]')).Count
    if ($headingCount -ne 1) {
      throw "Expected exactly one v0.1.1-preview.1 section after re-run, found $headingCount"
    }
    if ($changelog -notmatch '## \[Unreleased\]') {
      throw "Unreleased heading was removed"
    }

    # Auto-detect of the previous tag should produce the same commit set.
    & $scriptPath `
      -Tag "v0.1.1-preview.1" `
      -Repository "KingYoSun/kukuri" `
      -ChangelogPath $changelogPath `
      -SectionOutputPath $sectionPath `
      -Date "2026-06-15" | Out-Null

    $autoSection = Get-Content -LiteralPath $sectionPath -Raw -Encoding UTF8
    if ($autoSection -match '#100') {
      throw "Auto-detected previous tag included commits before v0.1.0-preview.1"
    }
    if ($autoSection -notmatch '#340') {
      throw "Auto-detected previous tag dropped the release commits"
    }

    Write-Host "update-changelog smoke test passed"
  }
  finally {
    Pop-Location
  }
}
finally {
  if (Test-Path -LiteralPath $workDir) {
    # .git can hold read-only objects on Windows; clear the attribute first.
    Get-ChildItem -LiteralPath $workDir -Recurse -Force -ErrorAction SilentlyContinue |
      ForEach-Object { $_.Attributes = 'Normal' }
    Remove-Item -LiteralPath $workDir -Recurse -Force -ErrorAction SilentlyContinue
  }
}
