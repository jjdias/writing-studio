# Make the icon files in this folder from icon.svg, on Windows, with what
# Windows already has: Edge renders the SVG, and .NET scales it and writes
# the PNGs, icon.ico (16 to 256 px, as PNGs), and window-64.rgba (the
# window's icon as raw RGBA pixels, which the program takes without an image
# decoder). Run it after changing icon.svg:
#
#     powershell -ExecutionPolicy Bypass -File apps/desktop/icons/make-icons.ps1
param([string]$Out = $PSScriptRoot)
# Stop at the first error, before a file is overwritten with nothing.
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$edge = @("${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe", "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe") |
    Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $edge) { throw "Microsoft Edge is needed to draw icon.svg" }
$work = Join-Path ([System.IO.Path]::GetTempPath()) "writing-studio-icon"
New-Item -ItemType Directory -Force $work | Out-Null
$svg = (Join-Path $Out "icon.svg") -replace '\\', '/'
$page = Join-Path $work "icon.html"
Set-Content -Path $page -Encoding utf8 -Value "<!doctype html><html><body style='margin:0;background:transparent'><img src='file:///$svg' width='512' height='512' style='display:block'></body></html>"
$Source = Join-Path $work "icon-512.png"
Remove-Item $Source -ErrorAction SilentlyContinue
# Edge reports on stderr, which this script would take as an error, so it
# runs as its own process and the script waits for it.
$drawing = @("--headless=new", "--disable-gpu", "--hide-scrollbars", "--default-background-color=00000000",
    "--window-size=512,512", "--screenshot=`"$Source`"", "file:///$($page -replace '\\', '/')")
Start-Process -FilePath $edge -ArgumentList $drawing -Wait -WindowStyle Hidden
if (-not (Test-Path $Source) -or (Get-Item $Source).Length -eq 0) { throw "Edge did not draw icon.svg; the icons are unchanged" }

$big = [System.Drawing.Bitmap]::FromFile($Source)
$corner = $big.GetPixel(0, 0)
"corner alpha: $($corner.A)"

function Resize([System.Drawing.Bitmap]$image, [int]$size) {
    $bitmap = New-Object System.Drawing.Bitmap($size, $size, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bitmap)
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $attributes = New-Object System.Drawing.Imaging.ImageAttributes
    $attributes.SetWrapMode([System.Drawing.Drawing2D.WrapMode]::TileFlipXY)
    $g.DrawImage($image, (New-Object System.Drawing.Rectangle(0, 0, $size, $size)), 0, 0, $image.Width, $image.Height, [System.Drawing.GraphicsUnit]::Pixel, $attributes)
    $g.Dispose()
    return $bitmap
}

function PngBytes([System.Drawing.Bitmap]$bitmap) {
    $stream = New-Object System.IO.MemoryStream
    $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png)
    return , $stream.ToArray()
}

# The PNGs the packages ask for.
foreach ($size in 32, 128, 256, 512) {
    $bitmap = if ($size -eq 512) { $big } else { Resize $big $size }
    [System.IO.File]::WriteAllBytes((Join-Path $Out "$size.png"), (PngBytes $bitmap))
}

# A Windows icon holding each size as a PNG, which Windows Vista and later read.
$sizes = 16, 24, 32, 48, 64, 128, 256
$images = foreach ($size in $sizes) { , (PngBytes (Resize $big $size)) }
$ico = New-Object System.IO.MemoryStream
$w = New-Object System.IO.BinaryWriter($ico)
$w.Write([UInt16]0); $w.Write([UInt16]1); $w.Write([UInt16]$sizes.Count)
$offset = 6 + 16 * $sizes.Count
for ($i = 0; $i -lt $sizes.Count; $i++) {
    $edge = if ($sizes[$i] -ge 256) { 0 } else { $sizes[$i] }
    $w.Write([Byte]$edge); $w.Write([Byte]$edge); $w.Write([Byte]0); $w.Write([Byte]0)
    $w.Write([UInt16]1); $w.Write([UInt16]32)
    $w.Write([UInt32]$images[$i].Length); $w.Write([UInt32]$offset)
    $offset += $images[$i].Length
}
foreach ($image in $images) { $w.Write($image) }
$w.Flush()
[System.IO.File]::WriteAllBytes((Join-Path $Out "icon.ico"), $ico.ToArray())

# The window's icon as raw RGBA pixels, which the window takes without an
# image decoder in the program.
$size = 64
$bitmap = Resize $big $size
$rect = New-Object System.Drawing.Rectangle(0, 0, $size, $size)
$data = $bitmap.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
$bgra = New-Object byte[] ($size * $size * 4)
[System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $bgra, 0, $bgra.Length)
$bitmap.UnlockBits($data)
$rgba = New-Object byte[] $bgra.Length
for ($i = 0; $i -lt $bgra.Length; $i += 4) {
    $rgba[$i] = $bgra[$i + 2]; $rgba[$i + 1] = $bgra[$i + 1]; $rgba[$i + 2] = $bgra[$i]; $rgba[$i + 3] = $bgra[$i + 3]
}
[System.IO.File]::WriteAllBytes((Join-Path $Out "window-64.rgba"), $rgba)
Get-ChildItem $Out | Select-Object Name, Length | Format-Table -AutoSize | Out-String
