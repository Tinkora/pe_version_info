param(
    [Parameter(Mandatory = $true)]
    [string]$PeviPath,
    [string]$EvidencePath = "artifacts/windows-native-evidence.json"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Assert-Condition {
    param(
        [Parameter(Mandatory = $true)]
        [bool]$Condition,
        [Parameter(Mandatory = $true)]
        [string]$Message
    )

    if (-not $Condition) {
        throw $Message
    }
}

function Invoke-Pevi {
    param([Parameter(Mandatory = $true)][string[]]$Arguments)

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = (Resolve-Path -LiteralPath $PeviPath).Path
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    foreach ($argument in $Arguments) {
        [void]$startInfo.ArgumentList.Add($argument)
    }

    $process = [System.Diagnostics.Process]::Start($startInfo)
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    if (-not $process.WaitForExit(30000)) {
        $process.Kill($true)
        throw "pevi exceeded the 30-second acceptance timeout"
    }
    $stdout = $stdoutTask.GetAwaiter().GetResult()
    $stderr = $stderrTask.GetAwaiter().GetResult()
    [pscustomobject]@{
        ExitCode = $process.ExitCode
        Stdout = $stdout.Trim()
        Stderr = $stderr.Trim()
    }
}

function Assert-PeviSuccess {
    param([Parameter(Mandatory = $true)]$Result)

    Assert-Condition ($Result.ExitCode -eq 0) "pevi failed: $($Result.Stderr) $($Result.Stdout)"
    $document = $Result.Stdout | ConvertFrom-Json -Depth 32
    Assert-Condition ($document.ok -eq $true) "pevi returned an unsuccessful JSON envelope"
    return $document
}

function Assert-PeviError {
    param(
        [Parameter(Mandatory = $true)]$Result,
        [Parameter(Mandatory = $true)][string]$Code
    )

    Assert-Condition ($Result.ExitCode -ne 0) "pevi unexpectedly accepted an unsafe request"
    $document = $Result.Stdout | ConvertFrom-Json -Depth 32
    Assert-Condition ($document.ok -eq $false) "pevi error envelope reported success"
    Assert-Condition ($document.errors[0].code -eq $Code) "expected $Code, got $($document.errors[0].code)"
}

function Write-Utf8File {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Content
    )

    Set-Content -LiteralPath $Path -Value $Content -Encoding utf8NoBOM
}

function New-TestPng {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][System.Drawing.Color]$Color
    )

    $bitmap = [System.Drawing.Bitmap]::new(96, 48)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.Clear($Color)
        $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $graphics.Dispose()
        $bitmap.Dispose()
    }
}

function New-CodeSigningCertificate {
    $rootRsa = [System.Security.Cryptography.RSA]::Create(2048)
    $leafRsa = [System.Security.Cryptography.RSA]::Create(2048)
    try {
        $rootRequest = [System.Security.Cryptography.X509Certificates.CertificateRequest]::new(
            "CN=Tinkora PEVI CI Evidence Root",
            $rootRsa,
            [System.Security.Cryptography.HashAlgorithmName]::SHA256,
            [System.Security.Cryptography.RSASignaturePadding]::Pkcs1
        )
        $rootUsage = [System.Security.Cryptography.X509Certificates.X509KeyUsageFlags]::KeyCertSign -bor
            [System.Security.Cryptography.X509Certificates.X509KeyUsageFlags]::CrlSign
        $rootRequest.CertificateExtensions.Add(
            [System.Security.Cryptography.X509Certificates.X509BasicConstraintsExtension]::new($true, $true, 1, $true)
        )
        $rootRequest.CertificateExtensions.Add(
            [System.Security.Cryptography.X509Certificates.X509KeyUsageExtension]::new($rootUsage, $true)
        )
        $rootCertificate = $rootRequest.CreateSelfSigned(
            [DateTimeOffset]::UtcNow.AddMinutes(-5),
            [DateTimeOffset]::UtcNow.AddHours(2)
        )

        $leafRequest = [System.Security.Cryptography.X509Certificates.CertificateRequest]::new(
            "CN=Tinkora PEVI CI Evidence",
            $leafRsa,
            [System.Security.Cryptography.HashAlgorithmName]::SHA256,
            [System.Security.Cryptography.RSASignaturePadding]::Pkcs1
        )
        $leafUsage = [System.Security.Cryptography.X509Certificates.X509KeyUsageFlags]::DigitalSignature
        $leafRequest.CertificateExtensions.Add(
            [System.Security.Cryptography.X509Certificates.X509BasicConstraintsExtension]::new($false, $false, 0, $true)
        )
        $leafRequest.CertificateExtensions.Add(
            [System.Security.Cryptography.X509Certificates.X509KeyUsageExtension]::new($leafUsage, $true)
        )
        $oids = [System.Security.Cryptography.OidCollection]::new()
        [void]$oids.Add([System.Security.Cryptography.Oid]::new("1.3.6.1.5.5.7.3.3"))
        $leafRequest.CertificateExtensions.Add(
            [System.Security.Cryptography.X509Certificates.X509EnhancedKeyUsageExtension]::new($oids, $true)
        )
        $leafPublicCertificate = $leafRequest.Create(
            $rootCertificate,
            [DateTimeOffset]::UtcNow.AddMinutes(-5),
            $rootCertificate.NotAfter.AddMinutes(-1),
            [Guid]::NewGuid().ToByteArray()
        )
        [pscustomobject]@{
            Root = $rootCertificate
            Leaf = [PeviNativeIcon]::AttachPrivateKey($leafPublicCertificate, $leafRsa)
        }
    } catch {
        throw
    } finally {
        $rootRsa.Dispose()
        $leafRsa.Dispose()
    }
}

function Install-CodeSigningCertificate {
    param([Parameter(Mandatory = $true)]$Certificate)

    $password = [Guid]::NewGuid().ToString("N")
    $pkcs12 = $Certificate.Export(
        [System.Security.Cryptography.X509Certificates.X509ContentType]::Pfx,
        $password
    )
    $flags = [System.Security.Cryptography.X509Certificates.X509KeyStorageFlags]::UserKeySet -bor
        [System.Security.Cryptography.X509Certificates.X509KeyStorageFlags]::PersistKeySet
    $persistedCertificate = [System.Security.Cryptography.X509Certificates.X509Certificate2]::new(
        $pkcs12,
        $password,
        $flags
    )
    $store = [System.Security.Cryptography.X509Certificates.X509Store]::new(
        [System.Security.Cryptography.X509Certificates.StoreName]::My,
        [System.Security.Cryptography.X509Certificates.StoreLocation]::CurrentUser
    )
    try {
        $store.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
        $store.Add($persistedCertificate)
    } finally {
        $store.Dispose()
        $persistedCertificate.Dispose()
        [Array]::Clear($pkcs12, 0, $pkcs12.Length)
    }

    $store = [System.Security.Cryptography.X509Certificates.X509Store]::new(
        [System.Security.Cryptography.X509Certificates.StoreName]::My,
        [System.Security.Cryptography.X509Certificates.StoreLocation]::CurrentUser
    )
    try {
        $store.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadOnly)
        foreach ($matching in $store.Certificates.Find(
            [System.Security.Cryptography.X509Certificates.X509FindType]::FindByThumbprint,
            $Certificate.Thumbprint,
            $false
        )) {
            if ($matching.HasPrivateKey) {
                return $matching
            }
        }
        throw "persisted code-signing certificate has no private key"
    } finally {
        $store.Dispose()
    }
}

function Remove-CodeSigningCertificate {
    param([Parameter(Mandatory = $true)][string]$Thumbprint)

    $store = [System.Security.Cryptography.X509Certificates.X509Store]::new(
        [System.Security.Cryptography.X509Certificates.StoreName]::My,
        [System.Security.Cryptography.X509Certificates.StoreLocation]::CurrentUser
    )
    try {
        $store.Open([System.Security.Cryptography.X509Certificates.OpenFlags]::ReadWrite)
        foreach ($matching in $store.Certificates.Find(
            [System.Security.Cryptography.X509Certificates.X509FindType]::FindByThumbprint,
            $Thumbprint,
            $false
        )) {
            $store.Remove($matching)
        }
    } finally {
        $store.Dispose()
    }
}

function Assert-CodeSigningCertificateChain {
    param(
        [Parameter(Mandatory = $true)]$Leaf,
        [Parameter(Mandatory = $true)]$Root
    )

    $chain = [System.Security.Cryptography.X509Certificates.X509Chain]::new()
    try {
        $chain.ChainPolicy.TrustMode = [System.Security.Cryptography.X509Certificates.X509ChainTrustMode]::CustomRootTrust
        $chain.ChainPolicy.CustomTrustStore.Add($Root)
        $chain.ChainPolicy.ExtraStore.Add($Root)
        $chain.ChainPolicy.RevocationMode = [System.Security.Cryptography.X509Certificates.X509RevocationMode]::NoCheck
        Assert-Condition $chain.Build($Leaf) (
            "custom Authenticode certificate chain failed: " +
            (($chain.ChainStatus | ForEach-Object { $_.StatusInformation.Trim() }) -join "; ")
        )
    } finally {
        $chain.Dispose()
    }
}

function Get-NativeVersion {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$ExpectedFileVersion,
        [Parameter(Mandatory = $true)][string]$ExpectedProductVersion,
        [Parameter(Mandatory = $true)][string]$ExpectedProductName
    )

    $version = [System.Diagnostics.FileVersionInfo]::GetVersionInfo($Path)
    $fileVersion = "$($version.FileMajorPart).$($version.FileMinorPart).$($version.FileBuildPart).$($version.FilePrivatePart)"
    $productVersion = "$($version.ProductMajorPart).$($version.ProductMinorPart).$($version.ProductBuildPart).$($version.ProductPrivatePart)"
    Assert-Condition ($fileVersion -eq $ExpectedFileVersion) "Windows reported unexpected file version $fileVersion"
    Assert-Condition ($productVersion -eq $ExpectedProductVersion) "Windows reported unexpected product version $productVersion"
    Assert-Condition ($version.ProductName -eq $ExpectedProductName) "Windows reported unexpected product name $($version.ProductName)"
    [ordered]@{
        file_version = $fileVersion
        product_version = $productVersion
        product_name = $version.ProductName
    }
}

Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Security.Cryptography;
using System.Security.Cryptography.X509Certificates;

public static class PeviNativeIcon
{
    public const uint CERT_E_UNTRUSTEDROOT = 0x800B0109u;
    public const uint CERT_E_CHAINING = 0x800B010Au;
    public const uint TRUST_E_NOSIGNATURE = 0x800B0100u;

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct WinTrustFileInfo
    {
        public uint StructSize;
        [MarshalAs(UnmanagedType.LPWStr)]
        public string FilePath;
        public IntPtr FileHandle;
        public IntPtr KnownSubject;
    }

    [StructLayout(LayoutKind.Sequential)]
    private struct WinTrustData
    {
        public uint StructSize;
        public IntPtr PolicyCallbackData;
        public IntPtr SipClientData;
        public uint UiChoice;
        public uint RevocationChecks;
        public uint UnionChoice;
        public IntPtr FileInfo;
        public uint StateAction;
        public IntPtr StateData;
        public IntPtr UrlReference;
        public uint ProviderFlags;
        public uint UiContext;
    }

    [DllImport("wintrust.dll", CharSet = CharSet.Unicode, ExactSpelling = true)]
    private static extern uint WinVerifyTrust(
        IntPtr window,
        ref Guid actionId,
        ref WinTrustData trustData);

    public static X509Certificate2 AttachPrivateKey(X509Certificate2 certificate, RSA key)
    {
        return certificate.CopyWithPrivateKey(key);
    }

    public static uint VerifyAuthenticode(string fileName)
    {
        var fileInfo = new WinTrustFileInfo
        {
            StructSize = (uint)Marshal.SizeOf(typeof(WinTrustFileInfo)),
            FilePath = fileName,
            FileHandle = IntPtr.Zero,
            KnownSubject = IntPtr.Zero
        };
        IntPtr fileInfoPointer = Marshal.AllocHGlobal(Marshal.SizeOf(typeof(WinTrustFileInfo)));
        bool fileInfoWritten = false;
        try
        {
            Marshal.StructureToPtr(fileInfo, fileInfoPointer, false);
            fileInfoWritten = true;
            var trustData = new WinTrustData
            {
                StructSize = (uint)Marshal.SizeOf(typeof(WinTrustData)),
                PolicyCallbackData = IntPtr.Zero,
                SipClientData = IntPtr.Zero,
                UiChoice = 2,
                RevocationChecks = 0,
                UnionChoice = 1,
                FileInfo = fileInfoPointer,
                StateAction = 0,
                StateData = IntPtr.Zero,
                UrlReference = IntPtr.Zero,
                ProviderFlags = 0x00001010,
                UiContext = 0
            };
            var actionId = new Guid("00AAC56B-CD44-11d0-8CC2-00C04FC295EE");
            return WinVerifyTrust(IntPtr.Zero, ref actionId, ref trustData);
        }
        finally
        {
            if (fileInfoWritten)
            {
                Marshal.DestroyStructure(fileInfoPointer, typeof(WinTrustFileInfo));
            }
            Marshal.FreeHGlobal(fileInfoPointer);
        }
    }

    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    public static extern uint ExtractIconEx(
        string fileName,
        int iconIndex,
        [Out] IntPtr[] largeIcons,
        [Out] IntPtr[] smallIcons,
        uint iconCount);

    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool DestroyIcon(IntPtr icon);
}
"@

function Get-NativeIconEvidence {
    param([Parameter(Mandatory = $true)][string]$Path)

    $count = [PeviNativeIcon]::ExtractIconEx($Path, -1, $null, $null, 0)
    Assert-Condition ($count -gt 0) "Windows shell reported no icon resources"
    $large = [IntPtr[]]::new(1)
    $small = [IntPtr[]]::new(1)
    $extracted = [PeviNativeIcon]::ExtractIconEx($Path, 0, $large, $small, 1)
    try {
        Assert-Condition ($extracted -gt 0 -and $extracted -ne [uint32]::MaxValue) "Windows shell could not extract the main icon: ExtractIconEx returned $extracted"
        Assert-Condition ($large[0] -ne [IntPtr]::Zero) "Windows shell did not return a large icon handle"
        Assert-Condition ($small[0] -ne [IntPtr]::Zero) "Windows shell did not return a small icon handle"
        return [ordered]@{
            group_count = $count
            extraction_return = $extracted
            large_handle_present = $true
            small_handle_present = $true
        }
    } finally {
        foreach ($handle in @($large[0], $small[0])) {
            if ($handle -ne [IntPtr]::Zero) {
                [void][PeviNativeIcon]::DestroyIcon($handle)
            }
        }
    }
}

$repositoryRoot = (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot "..")).Path
$temporaryRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("pevi-windows-evidence-" + [Guid]::NewGuid().ToString("N"))
$generatedCertificate = $null
$rootCertificate = $null
$signingCertificate = $null
$codeSigningCertificateThumbprint = $null
New-Item -ItemType Directory -Path $temporaryRoot | Out-Null

try {
    Write-Host "Windows evidence phase: create icon fixtures"
    $firstIcon = Join-Path $temporaryRoot "first.png"
    $secondIcon = Join-Path $temporaryRoot "second.png"
    New-TestPng -Path $firstIcon -Color ([System.Drawing.Color]::FromArgb(255, 20, 120, 210))
    New-TestPng -Path $secondIcon -Color ([System.Drawing.Color]::FromArgb(255, 220, 70, 70))

    $initialExe = Join-Path $temporaryRoot "existing-icon.exe"
    $initialConfig = Join-Path $temporaryRoot "initial.toml"
    Write-Utf8File -Path $initialConfig -Content @"
schema_version = 1
input = "$((Join-Path $repositoryRoot "fixtures/pe64_unsigned.exe").Replace('\', '/'))"
output = "$($initialExe.Replace('\', '/'))"

[policy]
overwrite_output = false
preserve_unknown_strings = true

[version]
file_version = "1.2.3.4"
product_version = "5.6.7.8"
language = "en-US"
code_page = 1200

[version.strings]
ProductName = "Tinkora PEVI Initial"

[icon]
source = "$($firstIcon.Replace('\', '/'))"
fit = "contain"
background = "transparent"
target_sizes = [16, 32, 48, 256]
"@
    Write-Host "Windows evidence phase: mutate and inspect unsigned EXE"
    [void](Assert-PeviSuccess (Invoke-Pevi @("apply", "--config", $initialConfig, "--format", "json")))
    [void](Assert-PeviSuccess (Invoke-Pevi @("verify", "--input", $initialExe, "--config", $initialConfig, "--format", "json")))
    $initialNativeVersion = Get-NativeVersion $initialExe "1.2.3.4" "5.6.7.8" "Tinkora PEVI Initial"
    $initialIconEvidence = Get-NativeIconEvidence $initialExe

    Write-Host "Windows evidence phase: create signing certificate"
    $signedExe = Join-Path $temporaryRoot "signed-existing-icon.exe"
    Copy-Item -LiteralPath $initialExe -Destination $signedExe
    $certificateChain = New-CodeSigningCertificate
    $rootCertificate = $certificateChain.Root
    $generatedCertificate = $certificateChain.Leaf
    Assert-Condition $generatedCertificate.HasPrivateKey "generated signing certificate has no private key"
    Write-Host "Windows evidence phase: persist signing private key"
    $signingCertificate = Install-CodeSigningCertificate $generatedCertificate
    Assert-Condition $signingCertificate.HasPrivateKey "installed signing certificate has no private key"
    $codeSigningCertificateThumbprint = $signingCertificate.Thumbprint
    Write-Host "Windows evidence phase: sign without mutating trust stores"
    $signingResult = Set-AuthenticodeSignature -FilePath $signedExe -Certificate $signingCertificate -HashAlgorithm SHA256
    $beforeEditSignature = Get-AuthenticodeSignature -FilePath $signedExe
    $beforeEditWinTrust = [PeviNativeIcon]::VerifyAuthenticode($signedExe)
    Assert-Condition ($beforeEditSignature.Status -eq "UnknownError") (
        "PowerShell did not report the expected untrusted-root status before the resource edit: " +
        "status=$($beforeEditSignature.Status) status_message=$($beforeEditSignature.StatusMessage)"
    )
    $beforeEditTrustFailures = @(
        [PeviNativeIcon]::CERT_E_UNTRUSTEDROOT,
        [PeviNativeIcon]::CERT_E_CHAINING
    )
    Assert-Condition ($beforeEditTrustFailures -contains $beforeEditWinTrust) (
        "WinVerifyTrust did not report an expected untrusted-chain failure before the resource edit: " +
        ("status=0x{0:X8}" -f $beforeEditWinTrust)
    )
    Assert-Condition ($beforeEditSignature.SignerCertificate.Thumbprint -eq $signingCertificate.Thumbprint) (
        "Authenticode signer certificate was not embedded: " +
        "set_status=$($signingResult.Status) set_message=$($signingResult.StatusMessage)"
    )
    Assert-CodeSigningCertificateChain $beforeEditSignature.SignerCertificate $rootCertificate

    Write-Host "Windows evidence phase: enforce signed-input policy"
    $mutatedExe = Join-Path $temporaryRoot "signed-mutated.exe"
    $signedConfig = Join-Path $temporaryRoot "signed.toml"
    Write-Utf8File -Path $signedConfig -Content @"
schema_version = 1
input = "$($signedExe.Replace('\', '/'))"
output = "$($mutatedExe.Replace('\', '/'))"

[policy]
overwrite_output = false
preserve_unknown_strings = true

[version]
file_version = "9.8.7.6"
product_version = "5.4.3.2"
language = "en-US"
code_page = 1200

[version.strings]
ProductName = "Tinkora PEVI Mutated"

[icon]
source = "$($secondIcon.Replace('\', '/'))"
fit = "contain"
background = "transparent"
target_sizes = [16, 32, 48, 256]
"@
    Assert-PeviError (Invoke-Pevi @("apply", "--config", $signedConfig, "--format", "json")) "signed_input_rejected"
    Assert-PeviError (Invoke-Pevi @("apply", "--config", $signedConfig, "--format", "json", "--allow-signed-input")) "signature_invalidation_not_acknowledged"
    $mutated = Assert-PeviSuccess (Invoke-Pevi @(
        "apply", "--config", $signedConfig, "--format", "json",
        "--allow-signed-input", "--acknowledge-signature-invalidation"
    ))
    Assert-Condition ($mutated.data.signature.input_certificate_table_present -eq $true) "apply did not report the signed input"
    Assert-Condition ($mutated.data.signature.signature_invalidated_by_edit -eq $true) "apply did not report signature invalidation"
    [void](Assert-PeviSuccess (Invoke-Pevi @("verify", "--input", $mutatedExe, "--config", $signedConfig, "--format", "json")))
    $afterEditSignature = Get-AuthenticodeSignature -FilePath $mutatedExe
    $afterEditWinTrust = [PeviNativeIcon]::VerifyAuthenticode($mutatedExe)
    Assert-Condition ($afterEditSignature.Status -eq "NotSigned") (
        "resource edit did not remove the Authenticode signature as expected: " +
        "status=$($afterEditSignature.Status) status_message=$($afterEditSignature.StatusMessage)"
    )
    Assert-Condition ($afterEditWinTrust -eq [PeviNativeIcon]::TRUST_E_NOSIGNATURE) (
        "WinVerifyTrust did not report the expected absent post-edit signature: " +
        ("status=0x{0:X8}" -f $afterEditWinTrust)
    )
    $mutatedNativeVersion = Get-NativeVersion $mutatedExe "9.8.7.6" "5.4.3.2" "Tinkora PEVI Mutated"
    $mutatedIconEvidence = Get-NativeIconEvidence $mutatedExe

    Write-Host "Windows evidence phase: mutate and inspect DLL"
    $dllSource = Join-Path $temporaryRoot "fixture.rs"
    $unsignedDll = Join-Path $temporaryRoot "fixture.dll"
    Write-Utf8File -Path $dllSource -Content @"
#![crate_type = "cdylib"]

#[unsafe(no_mangle)]
pub extern "C" fn pevi_fixture_value() -> u32 { 42 }
"@
    & rustc --edition 2024 --crate-type cdylib $dllSource -o $unsignedDll
    Assert-Condition ($LASTEXITCODE -eq 0) "rustc could not build the temporary DLL fixture"
    $mutatedDll = Join-Path $temporaryRoot "fixture-versioned.dll"
    $dllConfig = Join-Path $temporaryRoot "dll.toml"
    Write-Utf8File -Path $dllConfig -Content @"
schema_version = 1
input = "$($unsignedDll.Replace('\', '/'))"
output = "$($mutatedDll.Replace('\', '/'))"

[policy]
overwrite_output = false
preserve_unknown_strings = true

[version]
file_version = "2.3.4.5"
product_version = "6.7.8.9"
language = "en-US"
code_page = 1200

[version.strings]
ProductName = "Tinkora PEVI DLL"
"@
    [void](Assert-PeviSuccess (Invoke-Pevi @("inspect", "--input", $unsignedDll, "--format", "json")))
    [void](Assert-PeviSuccess (Invoke-Pevi @("apply", "--config", $dllConfig, "--format", "json")))
    [void](Assert-PeviSuccess (Invoke-Pevi @("verify", "--input", $mutatedDll, "--config", $dllConfig, "--format", "json")))
    $dllNativeVersion = Get-NativeVersion $mutatedDll "2.3.4.5" "6.7.8.9" "Tinkora PEVI DLL"

    Write-Host "Windows evidence phase: write evidence"
    $evidence = [ordered]@{
        schema_version = 1
        platform = "windows"
        authenticode = [ordered]@{
            before_edit = $beforeEditSignature.Status.ToString()
            after_edit = $afterEditSignature.Status.ToString()
            before_edit_winverifytrust = ("0x{0:X8}" -f $beforeEditWinTrust)
            after_edit_winverifytrust = ("0x{0:X8}" -f $afterEditWinTrust)
            before_edit_digest_intact = $true
            before_edit_custom_chain_valid = $true
            before_edit_system_trusted = $false
            after_edit_signature_absent = $true
            trust_store_mutated = $false
            default_rejected = $true
            partial_acknowledgement_rejected = $true
            explicit_override_required = $true
        }
        existing_icon_exe = [ordered]@{
            before_edit = $initialNativeVersion
            after_edit = $mutatedNativeVersion
            native_icon_before = $initialIconEvidence
            native_icon_after = $mutatedIconEvidence
        }
        dll = $dllNativeVersion
    }
    $evidenceParent = Split-Path -Parent $EvidencePath
    if ($evidenceParent) {
        New-Item -ItemType Directory -Force -Path $evidenceParent | Out-Null
    }
    $evidence | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $EvidencePath -Encoding utf8NoBOM
    Write-Host "Windows native evidence passed: $EvidencePath"
} finally {
    if ($null -ne $codeSigningCertificateThumbprint) {
        Remove-CodeSigningCertificate $codeSigningCertificateThumbprint
    }
    if ($null -ne $signingCertificate) {
        $signingCertificate.Dispose()
    }
    if ($null -ne $generatedCertificate) {
        $generatedCertificate.Dispose()
    }
    if ($null -ne $rootCertificate) {
        $rootCertificate.Dispose()
    }
    Remove-Item -LiteralPath $temporaryRoot -Recurse -Force -ErrorAction SilentlyContinue
}
