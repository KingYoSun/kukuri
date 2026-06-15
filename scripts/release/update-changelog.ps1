param(
  [Parameter(Mandatory = $true)]
  [string]$Tag,
  [Parameter(Mandatory = $true)]
  [string]$Repository,
  [string]$PreviousTag,
  [string]$ChangelogPath,
  [string]$SectionOutputPath,
  [string]$Date
)

$ErrorActionPreference = "Stop"

if (-not $ChangelogPath) {
  $ChangelogPath = Join-Path $PSScriptRoot "..\..\CHANGELOG.md"
}
$ChangelogPath = (Resolve-Path -LiteralPath $ChangelogPath).Path

if (-not $Date) {
  $Date = (Get-Date).ToUniversalTime().ToString("yyyy-MM-dd")
}

function Invoke-Git {
  param([string[]]$Arguments)
  $output = & git @Arguments 2>$null
  return @{ ExitCode = $LASTEXITCODE; Output = $output }
}

# Resolve the end of the commit range. Prefer the tag when it already exists
# (the release workflow pushes the tag before this runs); fall back to HEAD so
# the script also works during local dry runs before the tag is created.
$tagRef = "refs/tags/$Tag"
$tagExists = (Invoke-Git @("rev-parse", "--verify", "--quiet", $tagRef)).ExitCode -eq 0
$endRef = if ($tagExists) { $Tag } else { "HEAD" }

# Resolve the previous release tag if it was not supplied explicitly.
if (-not $PreviousTag) {
  $describe = Invoke-Git @("describe", "--tags", "--abbrev=0", "$endRef^")
  if ($describe.ExitCode -eq 0 -and $describe.Output) {
    $PreviousTag = ($describe.Output | Select-Object -First 1).Trim()
  }
}

$range = if ($PreviousTag) { "$PreviousTag..$endRef" } else { $endRef }

$logResult = Invoke-Git @("log", $range, "--no-merges", "--pretty=format:%s")
if ($logResult.ExitCode -ne 0) {
  throw "git log failed for range '$range'. Ensure the repository has full history (fetch-depth: 0) and that '$PreviousTag' / '$endRef' exist."
}

$subjects = @($logResult.Output | Where-Object { $_ -and $_.Trim() -ne "" })

# Classify each commit subject by Conventional Commit type and extract the pull
# request references so they can be linked.
$features = New-Object System.Collections.Generic.List[string]
$fixes = New-Object System.Collections.Generic.List[string]
$other = New-Object System.Collections.Generic.List[string]

foreach ($subject in $subjects) {
  $line = $subject.Trim()

  $type = "other"
  $typeMatch = [regex]::Match($line, '^(?<type>[a-zA-Z]+)(\([^)]*\))?(?<breaking>!)?:\s*(?<desc>.*)$')
  if ($typeMatch.Success) {
    $type = $typeMatch.Groups['type'].Value.ToLowerInvariant()
    $description = $typeMatch.Groups['desc'].Value.Trim()
  }
  else {
    $description = $line
  }

  # Collect every (#NNN) reference, then strip them from the description text.
  $prNumbers = New-Object System.Collections.Generic.List[string]
  foreach ($m in [regex]::Matches($description, '\(#(?<num>\d+)\)')) {
    $prNumbers.Add($m.Groups['num'].Value)
  }
  $description = ([regex]::Replace($description, '\s*\(#\d+\)', '')).Trim()
  if (-not $description) { $description = $line }

  $links = ($prNumbers | ForEach-Object { "[#$_](https://github.com/$Repository/pull/$_)" }) -join ", "
  $entry = if ($links) { "- $description ($links)" } else { "- $description" }

  switch ($type) {
    "feat" { $features.Add($entry) }
    "fix" { $fixes.Add($entry) }
    default { $other.Add($entry) }
  }
}

if (($features.Count + $fixes.Count + $other.Count) -eq 0) {
  Write-Warning "No commits found for range '$range'; CHANGELOG not modified."
  return
}

# Build the section body for this tag.
$sectionLines = New-Object System.Collections.Generic.List[string]
$sectionLines.Add("## [$Tag] - $Date")
$sectionLines.Add("")
foreach ($group in @(
    @{ Title = "Features"; Items = $features },
    @{ Title = "Fixes"; Items = $fixes },
    @{ Title = "Other"; Items = $other }
  )) {
  if ($group.Items.Count -gt 0) {
    $sectionLines.Add("### $($group.Title)")
    $sectionLines.Add("")
    foreach ($item in $group.Items) { $sectionLines.Add($item) }
    $sectionLines.Add("")
  }
}
$section = ($sectionLines -join "`n").TrimEnd()

if ($SectionOutputPath) {
  $sectionDir = Split-Path -Parent $SectionOutputPath
  if ($sectionDir -and -not (Test-Path -LiteralPath $sectionDir)) {
    New-Item -ItemType Directory -Force -Path $sectionDir | Out-Null
  }
  Set-Content -LiteralPath $SectionOutputPath -Value $section -Encoding UTF8
  Write-Host "Wrote changelog section for $Tag to $SectionOutputPath"
}

# Insert the section into CHANGELOG.md just below the [Unreleased] heading.
$content = Get-Content -LiteralPath $ChangelogPath -Encoding UTF8
$lines = New-Object System.Collections.Generic.List[string]
$lines.AddRange([string[]]$content)

$unreleasedIndex = -1
for ($i = 0; $i -lt $lines.Count; $i++) {
  if ($lines[$i] -match '^##\s+\[Unreleased\]') { $unreleasedIndex = $i; break }
}
if ($unreleasedIndex -lt 0) {
  throw "Could not find an '## [Unreleased]' heading in $ChangelogPath"
}

# Idempotency: drop an existing section for the same tag before re-inserting.
$existingIndex = -1
for ($i = 0; $i -lt $lines.Count; $i++) {
  if ($lines[$i] -match ('^##\s+\[' + [regex]::Escape($Tag) + '\]')) { $existingIndex = $i; break }
}
if ($existingIndex -ge 0) {
  $end = $lines.Count
  for ($j = $existingIndex + 1; $j -lt $lines.Count; $j++) {
    if ($lines[$j] -match '^##\s+\[') { $end = $j; break }
  }
  $lines.RemoveRange($existingIndex, $end - $existingIndex)
}

# Find the insertion point: the first version heading after [Unreleased], or EOF.
$insertAt = $lines.Count
for ($i = $unreleasedIndex + 1; $i -lt $lines.Count; $i++) {
  if ($lines[$i] -match '^##\s+\[') { $insertAt = $i; break }
}

$block = New-Object System.Collections.Generic.List[string]
$block.Add("")
foreach ($sl in ($section -split "`n")) { $block.Add($sl) }
$block.Add("")

$lines.InsertRange($insertAt, $block)

# Collapse any run of 3+ blank lines that the insertion may have produced.
$normalized = New-Object System.Collections.Generic.List[string]
$blankRun = 0
foreach ($line in $lines) {
  if ($line.Trim() -eq "") {
    $blankRun++
    if ($blankRun -ge 2) { continue }
  }
  else {
    $blankRun = 0
  }
  $normalized.Add($line)
}

Set-Content -LiteralPath $ChangelogPath -Value ($normalized -join "`n") -Encoding UTF8
Write-Host "Updated $ChangelogPath with section for $Tag (range: $range)"
