# Rebuild the native garden sprites with layered foliage and stippled soil.
# Keeps the runtime's existing sprite names, sizes, and transparent backgrounds.
Add-Type -AssemblyName System.Drawing
$outputRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../crates/benilla-app/assets/ui/herbalism'))

function Write-Sprite($name, $size, $draw) {
    $bitmap = New-Object Drawing.Bitmap($size, $size)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    $graphics.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
    & $draw $graphics $size
    $stream = [IO.File]::Create((Join-Path $outputRoot "$name.tga"))
    $writer = New-Object IO.BinaryWriter($stream)
    try {
        # Uncompressed BGRA, top-left origin, eight alpha bits.
        $header = New-Object byte[] 18
        $header[2] = 2
        $header[12] = $size -band 255
        $header[13] = $size -shr 8
        $header[14] = $size -band 255
        $header[15] = $size -shr 8
        $header[16] = 32
        $header[17] = 40
        $writer.Write($header)
        for ($y = 0; $y -lt $size; $y++) {
            for ($x = 0; $x -lt $size; $x++) {
                $c = $bitmap.GetPixel($x, $y)
                if ($name -like 'bed_corner_*') {
                    $cx = if ($name.EndsWith('tl') -or $name.EndsWith('bl')) { $size } else { 0 }
                    $cy = if ($name.EndsWith('tl') -or $name.EndsWith('tr')) { $size } else { 0 }
                    if (($x - $cx) * ($x - $cx) + ($y - $cy) * ($y - $cy) -gt $size * $size) {
                        $c = [Drawing.Color]::Transparent
                    }
                }
                $writer.Write([byte]$c.B); $writer.Write([byte]$c.G)
                $writer.Write([byte]$c.R); $writer.Write([byte]$c.A)
            }
        }
    } finally {
        $writer.Dispose(); $graphics.Dispose(); $bitmap.Dispose()
    }
}

function Oval($g, $color, $x, $y, $w, $h) {
    $brush = New-Object Drawing.SolidBrush([Drawing.ColorTranslator]::FromHtml($color))
    try { $g.FillEllipse($brush, [single]$x, [single]$y, [single]$w, [single]$h) }
    finally { $brush.Dispose() }
}

foreach ($kind in @('hidden', 'wilted', 'thorn')) {
    $colors = switch ($kind) {
        hidden { @('#183510', '#254e1b', '#326525', '#477d32') }
        wilted { @('#353322', '#55513a', '#726a48', '#90835b') }
        thorn  { @('#421e16', '#682e20', '#8c412a', '#b25c37') }
    }
    foreach ($variant in 1..3) {
        Write-Sprite "bush_${kind}_$variant" 128 {
            param($g, $size)
            Oval $g '#211b0c' 12 100 106 14
            $spread = (0, 6, -3)[$variant - 1]
            Oval $g $colors[0] (15 - $spread) 52 (98 + 2 * $spread) 61
            Oval $g $colors[1] (21 - $spread) 44 52 51
            Oval $g $colors[1] (61 + $spread) 43 48 52
            Oval $g $colors[1] 42 (26 + $spread) 49 58
            Oval $g $colors[2] (26 - $spread) 47 32 29
            Oval $g $colors[2] 48 (31 + $spread) 31 31
            Oval $g $colors[2] (76 + $spread) 53 23 25
            Oval $g $colors[3] (32 - $spread) 49 14 14
            Oval $g $colors[3] 52 (34 + $spread) 13 12
            Oval $g $colors[3] (80 + $spread) 56 11 12
        }
    }
}

Write-Sprite 'bed_soil' 128 {
    param($g, $size)
    $g.Clear([Drawing.ColorTranslator]::FromHtml('#50371f'))
    $random = New-Object Random(435)
    for ($y = 0; $y -lt 128; $y += 8) {
        for ($x = 0; $x -lt 128; $x += 8) {
            $j = $random.Next(-1, 2)
            Oval $g '#342313' ($x + $j) ($y + 2) 4 5
            Oval $g '#735032' ($x + $j) ($y + 1) 2 2
        }
    }
}

foreach ($piece in @('edge_top','edge_bottom','edge_left','edge_right','corner_tl','corner_tr','corner_bl','corner_br')) {
    Write-Sprite "bed_$piece" 64 {
        param($g, $size)
        $g.Clear([Drawing.ColorTranslator]::FromHtml('#24431b'))
        $random = New-Object Random(482)
        for ($i = 0; $i -lt 100; $i++) {
            $color = @('#355f26','#487a32','#5b8b3c','#294e20')[$random.Next(4)]
            Oval $g $color ($random.Next(-4, 61)) ($random.Next(-4, 61)) 9 10
        }
    }
}
Write-Sprite 'bush_glow' 128 {
    param($g, $size)
    for ($radius = 60; $radius -ge 32; $radius -= 2) {
        $brush = New-Object Drawing.SolidBrush([Drawing.Color]::FromArgb(5, 255, 214, 82))
        $g.FillEllipse($brush, 64 - $radius, 68 - $radius, 2 * $radius, 2 * $radius)
        $brush.Dispose()
    }
    foreach ($point in @(@(30,40), @(86,46), @(42,87), @(96,79))) {
        Oval $g '#dfdd8d' $point[0] $point[1] 5 5
    }
}
Write-Output 'Styled 19 garden sprites; preserved all item glyphs and interaction targets.'
