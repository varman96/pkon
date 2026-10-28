<#
.SYNOPSIS  Per-user (HKCU, no admin) install / uninstall of the PKON WIC decoder.
.EXAMPLE   .\install.ps1 -Dll .\target\release\pkon_wic.dll
.EXAMPLE   .\install.ps1 -Uninstall
Only keys owned by this decoder are written/removed. Nothing under HKLM is touched.
#>
param([string]$Dll = (Join-Path $PSScriptRoot 'target\release\pkon_wic.dll'), [switch]$Uninstall)

$ErrorActionPreference = 'Stop'
$clsid     = '{A1D6F3E2-8B47-4F09-B5C3-2E9D7A10C648}'
$container = '{7B3C5A0E-4D1F-4C8A-9B62-5E0A1F3D7C24}'
$vendor    = '{B3A8F1D0-2C6E-4E5B-8F97-0D4C6A9E1B35}'
$catDecoder = '{7ED96837-96F0-4812-B211-F13C24117ED3}'   # CATID_WICBitmapDecoders
$thumbProv  = '{C7657C4A-9F68-40FA-A4DF-96BC08EB3551}'   # WIC photo thumbnail provider
$shellThumb = '{e357fccd-a995-4576-b01f-234630154e96}'   # IThumbnailProvider shellex handler key
$fmtGuids = @('{6FDDC324-4E03-4BFE-B185-3D77768DC908}',  # 8bppGray
              '{6FDDC324-4E03-4BFE-B185-3D77768DC90D}',  # 24bppRGB
              '{F5C7AD2D-6A8D-43DD-A7A8-A29935261AE9}')  # 32bppRGBA
$root = 'HKCU:\Software\Classes'

function Remove-Key($p) { if (Test-Path $p) { Remove-Item $p -Recurse -Force } }

if ($Uninstall) {
    Remove-Key "$root\CLSID\$clsid"
    Remove-Key "$root\CLSID\$catDecoder\Instance\$clsid"
    Remove-Key "$root\.pkon\ShellEx\$shellThumb"
    # Only remove .pkon if we created it (marker value) and nothing else is under it.
    if ((Test-Path "$root\.pkon") -and ((Get-ItemProperty "$root\.pkon").PkonWicInstalled -eq 1)) { Remove-Key "$root\.pkon" }
    Remove-Key "$root\PKON.Image"
    Write-Host 'PKON WIC decoder uninstalled (HKCU).'
    return
}

$Dll = (Resolve-Path $Dll).Path
function Set-Default($path, $value) { New-Item -Path $path -Force | Out-Null; Set-Item -Path $path -Value $value }

# COM class
Set-Default "$root\CLSID\$clsid" 'PKON2 WIC Decoder'
Set-Default "$root\CLSID\$clsid\InprocServer32" $Dll
Set-ItemProperty "$root\CLSID\$clsid\InprocServer32" -Name ThreadingModel -Value 'Both'

# WIC component registration (same values a built-in decoder has)
$k = "$root\CLSID\$clsid"
Set-ItemProperty $k -Name FriendlyName     -Value 'PKON2 Decoder'
Set-ItemProperty $k -Name Author           -Value 'nanoscope'
Set-ItemProperty $k -Name Vendor           -Value $vendor
Set-ItemProperty $k -Name Version          -Value '1.0.0.0'
Set-ItemProperty $k -Name SpecVersion      -Value '1.0.0.0'
Set-ItemProperty $k -Name ContainerFormat  -Value $container
Set-ItemProperty $k -Name FileExtensions   -Value '.pkon'
Set-ItemProperty $k -Name MimeTypes        -Value 'image/x-pkon'
Set-ItemProperty $k -Name ArbitrationPriority -Value 0 -Type DWord
Set-ItemProperty $k -Name SupportAnimation -Value 0 -Type DWord
Set-ItemProperty $k -Name SupportChromakey -Value 0 -Type DWord
Set-ItemProperty $k -Name SupportLossless  -Value 1 -Type DWord
Set-ItemProperty $k -Name SupportMultiframe -Value 0 -Type DWord
foreach ($f in $fmtGuids) { New-Item -Path "$k\Formats\$f" -Force | Out-Null }
# Signature: 0x89 'P' 'K' '2' at offset 0 (format magic), full byte mask.
New-Item -Path "$k\Patterns\0" -Force | Out-Null
$p = "$k\Patterns\0"
Set-ItemProperty $p -Name Position -Value 0 -Type DWord
Set-ItemProperty $p -Name Length   -Value 4 -Type DWord
Set-ItemProperty $p -Name Pattern  -Value ([byte[]](0x89,0x50,0x4B,0x32)) -Type Binary
Set-ItemProperty $p -Name Mask     -Value ([byte[]](0xFF,0xFF,0xFF,0xFF)) -Type Binary
# Make it enumerable as a WIC bitmap decoder.
Set-Default "$root\CLSID\$catDecoder\Instance\$clsid" ''
Set-ItemProperty "$root\CLSID\$catDecoder\Instance\$clsid" -Name CLSID -Value $clsid
Set-ItemProperty "$root\CLSID\$catDecoder\Instance\$clsid" -Name FriendlyName -Value 'PKON2 Decoder'

# File association (extension -> image type + thumbnail + ProgID), created only if absent.
if (-not (Test-Path "$root\.pkon")) { New-Item "$root\.pkon" -Force | Out-Null; Set-ItemProperty "$root\.pkon" -Name PkonWicInstalled -Value 1 -Type DWord }
Set-ItemProperty "$root\.pkon" -Name 'Content Type'  -Value 'image/x-pkon'
Set-ItemProperty "$root\.pkon" -Name PerceivedType   -Value 'image'
Set-Default "$root\.pkon\ShellEx\$shellThumb" $thumbProv
New-Item "$root\.pkon\OpenWithProgids" -Force | Out-Null
New-ItemProperty "$root\.pkon\OpenWithProgids" -Name 'PKON.Image' -Value ([byte[]]@()) -PropertyType Binary -Force | Out-Null
Set-Default "$root\PKON.Image" 'PKON2 Image'

# Tell the shell the associations changed.
Add-Type -Namespace W -Name S -MemberDefinition '[DllImport("shell32.dll")] public static extern void SHChangeNotify(int e,int f,IntPtr a,IntPtr b);'
[W.S]::SHChangeNotify(0x08000000, 0, [IntPtr]::Zero, [IntPtr]::Zero)
Write-Host "PKON WIC decoder installed for current user: $Dll"
