<#
Runs under Windows PowerShell 5.1 (powershell.exe): decodes each fixture through the *registered* WIC decoder
(WPF BitmapDecoder -> WIC component enumeration -> pkon_wic.dll) and compares every pixel byte with the .raw
reference written by the production Rust decoder. Malformed files must fail to decode.
  powershell.exe -NoProfile -File .\test.ps1
#>
param([string]$Dir = (Join-Path $PSScriptRoot 'target\fixtures'))
Add-Type -AssemblyName PresentationCore
$fail = 0
function Decode($path) {
    $uri = New-Object Uri($path)
    $d = [Windows.Media.Imaging.BitmapDecoder]::Create($uri, 'None', 'OnLoad')
    $f = $d.Frames[0]
    $bpp = $f.Format.BitsPerPixel
    $stride = [int](($f.PixelWidth * $bpp + 7) / 8)
    $buf = New-Object byte[] ($stride * $f.PixelHeight)
    $f.CopyPixels($buf, $stride, 0)
    [pscustomobject]@{ W = $f.PixelWidth; H = $f.PixelHeight; Format = "$($f.Format)"; Bytes = $buf; Container = $d.CodecInfo.ContainerFormat }
}
foreach ($name in 'rgb_c1','rgb_c2','rgb_c3','gray_c3','rgba_c3') {
    try {
        $r = Decode "$Dir\$name.pkon"
        $ref = [IO.File]::ReadAllBytes("$Dir\$name.raw")
        $ok = $r.Bytes.Length -eq $ref.Length
        if ($ok) { for ($i = 0; $i -lt $ref.Length; $i++) { if ($r.Bytes[$i] -ne $ref[$i]) { $ok = $false; break } } }
        # WPF may hand back Bgr24/Bgra32/Gray8; reorder to RGB(A) before comparing when it did.
        if (-not $ok -and $r.Format -match 'Bgr') {
            $n = if ($r.Format -eq 'Bgr24') { 3 } else { 4 }; $ok = $true
            for ($i = 0; $i -lt $ref.Length; $i += $n) { if ($r.Bytes[$i] -ne $ref[$i+2] -or $r.Bytes[$i+1] -ne $ref[$i+1] -or $r.Bytes[$i+2] -ne $ref[$i] -or ($n -eq 4 -and $r.Bytes[$i+3] -ne $ref[$i+3])) { $ok = $false; break } }
        }
        "{0,-10} {1}x{2} {3,-8} pixels-match-rust={4}" -f $name, $r.W, $r.H, $r.Format, $ok
        if (-not $ok) { $fail++ }
    } catch { "{0,-10} FAILED: {1}" -f $name, $_.Exception.InnerException.Message; $fail++ }
}
foreach ($f in @(Get-ChildItem $Dir -Filter 'bad_*.pkon') + @(Get-Item "$Dir\graya_c3.pkon")) {
    try { Decode $f.FullName | Out-Null; "{0,-28} UNEXPECTEDLY DECODED" -f $f.Name; $fail++ }
    catch { "{0,-28} rejected: {1}" -f $f.Name, ($_.Exception.InnerException.Message -replace '\s+', ' ') }
}
if ($fail) { "FAILURES: $fail"; exit 1 } else { 'ALL OK' }
