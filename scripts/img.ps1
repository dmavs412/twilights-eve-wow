# Look at part of an image without Pillow: crop a region, compare one region of two images side
# by side, or shrink an image to a JPEG small enough to upload. Paths are relative to the
# current directory or absolute. Read the output PNG with the Read tool.
#
#   powershell -NoProfile -File scripts/img.ps1 crop    -File a.png -Rect 450,700,1300,900 -Out c.png
#   powershell -NoProfile -File scripts/img.ps1 crop    -File a.png -Rect 320,530,100,70 -Scale 4 -Ticks -Out c.png
#   powershell -NoProfile -File scripts/img.ps1 compare -File old.png -Other new.png -Rect 660,330,200,100 -Scale 3 -Out c.png
#   powershell -NoProfile -File scripts/img.ps1 shrink  -File big.png -Width 1536 -Out small.jpg
#
# -Rect is x,y,width,height in source pixels. -Scale enlarges with nearest-neighbour, so pixels
# stay sharp. -Ticks draws a red tick every 10 source pixels along the top and left edges, so a
# position in the crop reads back as a source pixel. shrink keeps the aspect ratio when -Height
# is left out and writes JPEG at quality 90.
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ValidateSet('crop', 'compare', 'shrink')]
    [string]$Command,
    [Parameter(Mandatory = $true)][string]$File,
    [Parameter(Mandatory = $true)][string]$Out,
    [string]$Other,
    [string]$Rect,
    [int]$Scale = 1,
    [switch]$Ticks,
    [int]$Width,
    [int]$Height
)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
# -Rect arrives as one string under -File, so it is split into $R here: "x,y,w,h".
$R = if ($Rect) { [int[]]($Rect -split ',') } else { @() }

function Open-Image([string]$Path) {
    [System.Drawing.Image]::FromFile((Resolve-Path $Path).ProviderPath)
}

function Out-Path([string]$Path) {
    $ExecutionContext.SessionState.Path.GetUnresolvedProviderPathFromPSPath($Path)
}

# Draw region $R of $Src at $Scale onto $G with its top-left at ($X, 0), ticks included.
function Draw-Region($G, $Src, [int]$X) {
    $from = New-Object System.Drawing.Rectangle $R[0], $R[1], $R[2], $R[3]
    $to = New-Object System.Drawing.Rectangle $X, 0, ($R[2] * $Scale), ($R[3] * $Scale)
    $G.DrawImage($Src, $to, $from, [System.Drawing.GraphicsUnit]::Pixel)
    if ($Ticks) {
        $pen = New-Object System.Drawing.Pen ([System.Drawing.Color]::Red), 2
        for ($i = 0; $i -le $R[2]; $i += 10) { $G.DrawLine($pen, $X + $i * $Scale, 0, $X + $i * $Scale, 10) }
        for ($i = 0; $i -le $R[3]; $i += 10) { $G.DrawLine($pen, $X, $i * $Scale, $X + 10, $i * $Scale) }
        $pen.Dispose()
    }
}

function New-Canvas([int]$W, [int]$H) {
    $bmp = New-Object System.Drawing.Bitmap $W, $H
    $g = [System.Drawing.Graphics]::FromImage($bmp)
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::NearestNeighbor
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::Half
    return $bmp, $g
}

$src = Open-Image $File
try {
    switch ($Command) {
        'crop' {
            if ($R.Count -ne 4) { throw 'crop needs -Rect x,y,width,height' }
            $bmp, $g = New-Canvas ($R[2] * $Scale) ($R[3] * $Scale)
            Draw-Region $g $src 0
            $bmp.Save((Out-Path $Out), [System.Drawing.Imaging.ImageFormat]::Png)
            $g.Dispose(); $bmp.Dispose()
        }
        'compare' {
            if ($R.Count -ne 4) { throw 'compare needs -Rect x,y,width,height' }
            if (-not $Other) { throw 'compare needs -Other' }
            $second = Open-Image $Other
            $gap = 20
            $w = $R[2] * $Scale
            $bmp, $g = New-Canvas ($w * 2 + $gap) ($R[3] * $Scale)
            Draw-Region $g $src 0
            Draw-Region $g $second ($w + $gap)
            $bmp.Save((Out-Path $Out), [System.Drawing.Imaging.ImageFormat]::Png)
            $g.Dispose(); $bmp.Dispose(); $second.Dispose()
        }
        'shrink' {
            if (-not $Width) { throw 'shrink needs -Width' }
            if (-not $Height) { $Height = [int][Math]::Round($src.Height * $Width / $src.Width) }
            $bmp = New-Object System.Drawing.Bitmap $Width, $Height
            $g = [System.Drawing.Graphics]::FromImage($bmp)
            $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
            $g.DrawImage($src, 0, 0, $Width, $Height)
            $jpeg = [System.Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() |
                Where-Object { $_.MimeType -eq 'image/jpeg' }
            $q = New-Object System.Drawing.Imaging.EncoderParameters 1
            $q.Param[0] = New-Object System.Drawing.Imaging.EncoderParameter ([System.Drawing.Imaging.Encoder]::Quality), 90L
            $bmp.Save((Out-Path $Out), $jpeg, $q)
            $g.Dispose(); $bmp.Dispose()
        }
    }
} finally {
    $src.Dispose()
}
Write-Output "wrote $Out"
