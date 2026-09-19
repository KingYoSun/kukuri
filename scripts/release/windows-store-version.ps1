function ConvertTo-StoreVersion([string]$AppVersion) {
    # Do not silently collapse prereleases/build metadata to a released version.
    if ($AppVersion -cnotmatch '^(0|[1-9][0-9]{0,4})\.(0|[1-9][0-9]{0,4})\.(0|[1-9][0-9]{0,4})$') {
        throw 'Store packaging requires a stable major.minor.patch app version'
    }
    $parts = @($AppVersion.Split('.') | ForEach-Object { [int]$_ })
    if ($parts[0] -gt 65534 -or $parts[1] -gt 65535 -or $parts[2] -gt 65535) {
        throw 'App version exceeds the Store 16-bit version range after major + 1'
    }
    return '{0}.{1}.{2}.0' -f ($parts[0] + 1), $parts[1], $parts[2]
}
