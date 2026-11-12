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
Get-Content smoke-stdout.log
Get-Content smoke-stderr.log
@{ peak_working_set_bytes=$peak; observed_cpu_seconds=($cpuByPid.Values | Measure-Object -Sum).Sum; wall_seconds=$sw.Elapsed.TotalSeconds; sampling_ms=250; note='Process sampling includes model download/loading and sequential tests; not simultaneous Brave playback'; cpu=(Get-CimInstance Win32_Processor | Select-Object Name,NumberOfCores,NumberOfLogicalProcessors); os=(Get-CimInstance Win32_OperatingSystem | Select-Object Caption,Version,TotalVisibleMemorySize) } | ConvertTo-Json -Depth 4 | Set-Content -Encoding utf8 resource-smoke.json
exit $p.ExitCode
