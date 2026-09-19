# Windows shell resource qualifiers keep the existing logo free of an accent plate.
# https://learn.microsoft.com/windows/apps/design/iconography/app-icon-construction
function New-StoreShellIcons([string]$Source, [string]$Destination) {
    Add-Type -AssemblyName System.Drawing
    $original = [Drawing.Bitmap]::new($Source)
    try {
        foreach ($size in @(16, 20, 24, 30, 32, 36, 40, 48, 60, 64, 72, 80, 96, 256)) {
            $bitmap = [Drawing.Bitmap]::new($size, $size, [Drawing.Imaging.PixelFormat]::Format32bppArgb)
            try {
                $graphics = [Drawing.Graphics]::FromImage($bitmap)
                try {
                    $graphics.Clear([Drawing.Color]::Transparent)
                    $graphics.CompositingMode = [Drawing.Drawing2D.CompositingMode]::SourceCopy
                    $graphics.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
                    $graphics.PixelOffsetMode = [Drawing.Drawing2D.PixelOffsetMode]::HighQuality
                    $graphics.DrawImage($original, [Drawing.Rectangle]::new(0, 0, $size, $size))
                }
                finally { $graphics.Dispose() }
                foreach ($suffix in @('', '_altform-unplated', '_altform-lightunplated')) {
                    $name = "Square44x44Logo.targetsize-${size}${suffix}.png"
                    $path = Join-Path $Destination $name
                    if (Test-Path -LiteralPath $path) { throw "Shell icon already exists: $path" }
                    $bitmap.Save($path, [Drawing.Imaging.ImageFormat]::Png)
                    Write-Output $name
                }
            }
            finally { $bitmap.Dispose() }
        }
    }
    finally { $original.Dispose() }
}
