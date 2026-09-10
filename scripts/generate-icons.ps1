$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

function Write-NotchIcon([int]$size, [string]$path) {
    $bitmap = [System.Drawing.Bitmap]::new($size, $size)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
        $graphics.ScaleTransform($size / 1024.0, $size / 1024.0)
        $background = [System.Drawing.Drawing2D.GraphicsPath]::new()
        foreach ($arc in @(@(40,40,180),@(568,40,270),@(568,568,0),@(40,568,90))) {
            $background.AddArc($arc[0],$arc[1],416,416,$arc[2],90)
        }
        $background.CloseFigure()
        $brush = [System.Drawing.SolidBrush]::new([System.Drawing.ColorTranslator]::FromHtml('#111216'))
        $graphics.FillPath($brush,$background)
        $pen = [System.Drawing.Pen]::new([System.Drawing.Color]::White,112)
        $pen.StartCap = $pen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
        $pen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
        $points = [System.Drawing.PointF[]]@([System.Drawing.PointF]::new(320,704),[System.Drawing.PointF]::new(320,320),[System.Drawing.PointF]::new(704,704),[System.Drawing.PointF]::new(704,320))
        $graphics.DrawLines($pen,$points)
        $pen.Color = [System.Drawing.ColorTranslator]::FromHtml('#00FF88')
        $pen.Width = 64
        $graphics.DrawLine($pen,414,176,610,176)
        $bitmap.Save($path,[System.Drawing.Imaging.ImageFormat]::Png)
        $pen.Dispose(); $brush.Dispose(); $background.Dispose()
    } finally { $graphics.Dispose(); $bitmap.Dispose() }
}

$catalog = 'apps/macos/Sources/Assets.xcassets/AppIcon.appiconset'
$manifest = Get-Content "$catalog/Contents.json" -Raw | ConvertFrom-Json
foreach ($icon in $manifest.images) {
    $pixels = [int]($icon.size.Split('x')[0]) * [int]($icon.scale.TrimEnd('x'))
    Write-NotchIcon $pixels (Join-Path $PWD "$catalog/$($icon.filename)")
}
Write-NotchIcon 32 (Join-Path $PWD 'apps/windows/notch/icons/tray.png')
Write-NotchIcon 256 (Join-Path $PWD 'work/notch-icon.png')
$png = [System.IO.File]::ReadAllBytes((Join-Path $PWD 'work/notch-icon.png'))
$stream = [System.IO.File]::Create((Join-Path $PWD 'apps/windows/notch/icons/icon.ico'))
$writer = [System.IO.BinaryWriter]::new($stream)
try {
    $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]1)
    $writer.Write([byte[]]@(0,0,0,0)); $writer.Write([uint16]1); $writer.Write([uint16]32)
    $writer.Write([uint32]$png.Length); $writer.Write([uint32]22); $writer.Write($png)
} finally { $writer.Dispose() }
