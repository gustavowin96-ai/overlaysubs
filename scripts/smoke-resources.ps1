param([string]$TestExecutable = '')
$ErrorActionPreference = 'Stop'
$sw = [Diagnostics.Stopwatch]::StartNew()
$command = if ($TestExecutable) { $TestExecutable } else { 'cargo' }
$testArgs = if ($TestExecutable) { @('ptbr_models_smoke','--ignored','--nocapture','--test-threads=1') } else { @('test','--release','-p','overlaysubs','ptbr_models_smoke','--','--ignored','--nocapture','--test-threads=1') }
$p = Start-Process -FilePath $command -ArgumentList $testArgs -WindowStyle Hidden -PassThru -RedirectStandardOutput smoke-stdout.log -RedirectStandardError smoke-stderr.log
$peak = 0L
$cpuByPid = @{}
while (!$p.HasExited) {
    Get-Process -Name 'overlaysubs-*' -ErrorAction SilentlyContinue | ForEach-Object {
        $peak = [Math]::Max($peak, $_.WorkingSet64)
        $cpuByPid[$_.Id] = $_.TotalProcessorTime.TotalSeconds
    }
    Start-Sleep -Milliseconds 250
    $p.Refresh()
}
$p.WaitForExit()
if ($null -eq $p.ExitCode) { throw 'Cannot determine smoke-test exit code' }
Get-Content smoke-stdout.log
Get-Content smoke-stderr.log
$hostError = $null
$cpuInfo = $null
$osInfo = $null
try {
    $cpuInfo = Get-CimInstance Win32_Processor | Select-Object Name,NumberOfCores,NumberOfLogicalProcessors
    $osInfo = Get-CimInstance Win32_OperatingSystem | Select-Object Caption,Version,TotalVisibleMemorySize
} catch { $hostError = $_.Exception.Message }
@{ peak_working_set_bytes=$peak; observed_cpu_seconds=($cpuByPid.Values | Measure-Object -Sum).Sum; wall_seconds=$sw.Elapsed.TotalSeconds; sampling_ms=250; note='Whole process, including model loading and English/bilingual replay; not Brave playback'; cpu=$cpuInfo; os=$osInfo; host_metadata_error=$hostError } | ConvertTo-Json -Depth 4 | Set-Content -Encoding utf8 resource-smoke.json
exit $p.ExitCode
