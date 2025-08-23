# PowerShell script for Windows audio recording using SoundRecorder (uses default audio device)
# This script records and saves continuously, so the file is always available
param(
    [Parameter(Mandatory=$true)]
    [string]$OutputFile
)

try {
    # Create an empty file immediately to ensure it exists
    $emptyBytes = New-Object byte[] 1024
    [System.IO.File]::WriteAllBytes($OutputFile, $emptyBytes)
    Write-Host "Created placeholder file: $OutputFile"
    
    # Use Windows built-in SoundRecorder which respects default audio device
    # Create a temporary batch file to run SoundRecorder
    $tempBat = [System.IO.Path]::GetTempFileName() + ".bat"
    $soundRecorderCmd = @"
@echo off
echo Recording started...
timeout /t 1 /nobreak >nul
echo Initial recording saved
timeout /t 30 /nobreak >nul
"@
    
    [System.IO.File]::WriteAllText($tempBat, $soundRecorderCmd)
    
    # Try to use Windows Media Format SDK or fall back to PowerShell audio recording
    Add-Type -AssemblyName System.Windows.Forms
    Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public class WinAudio {
    [DllImport("winmm.dll", EntryPoint="mciSendStringA", CharSet=CharSet.Ansi)]
    public static extern int mciSendString(string lpstrCommand, StringBuilder lpstrReturnString, int uReturnLength, IntPtr hwndCallback);
}
"@

    $sb = New-Object System.Text.StringBuilder(255)
    
    # Open the default waveaudio input device
    $result = [WinAudio]::mciSendString("open new type waveaudio alias recsession", $sb, $sb.Capacity, [IntPtr]::Zero)
    if ($result -ne 0) {
        Write-Host "Failed to open default audio device. Error code: $result"
        exit 1
    }
    
    # Set recording format to 16kHz, 16-bit, mono (CD quality downsampled)
    [WinAudio]::mciSendString("set recsession time format ms", $sb, $sb.Capacity, [IntPtr]::Zero)
    [WinAudio]::mciSendString("set recsession bitspersample 16", $sb, $sb.Capacity, [IntPtr]::Zero)
    [WinAudio]::mciSendString("set recsession samplespersec 16000", $sb, $sb.Capacity, [IntPtr]::Zero)
    [WinAudio]::mciSendString("set recsession channels 1", $sb, $sb.Capacity, [IntPtr]::Zero)
    
    # Start recording from default input
    $result = [WinAudio]::mciSendString("record recsession", $sb, $sb.Capacity, [IntPtr]::Zero)
    if ($result -ne 0) {
        Write-Host "Failed to start recording from default device. Error code: $result"
        [WinAudio]::mciSendString("close recsession", $sb, $sb.Capacity, [IntPtr]::Zero)
        exit 1
    }
    
    Write-Host "Recording started from default audio input device..."
    
    # Wait a brief moment to capture some audio, then save immediately
    Start-Sleep -Milliseconds 300
    
    # Stop and save the initial recording to ensure file exists
    [WinAudio]::mciSendString("stop recsession", $sb, $sb.Capacity, [IntPtr]::Zero)
    $result = [WinAudio]::mciSendString("save recsession `"$OutputFile`"", $sb, $sb.Capacity, [IntPtr]::Zero)
    if ($result -eq 0) {
        Write-Host "Initial recording saved from default device"
    }
    
    # Resume recording and continue saving periodically
    [WinAudio]::mciSendString("record recsession", $sb, $sb.Capacity, [IntPtr]::Zero)
    
    $iteration = 1
    while ($true) {
        Start-Sleep -Milliseconds 500
        $iteration++
        
        # Every second, save the current recording
        if ($iteration % 2 -eq 0) {
            # Stop current recording
            [WinAudio]::mciSendString("stop recsession", $sb, $sb.Capacity, [IntPtr]::Zero)
            
            # Save the recording
            $result = [WinAudio]::mciSendString("save recsession `"$OutputFile`"", $sb, $sb.Capacity, [IntPtr]::Zero)
            if ($result -eq 0) {
                Write-Host "Recording saved from default device (iteration $iteration)"
            }
            
            # Resume recording to continue capturing
            [WinAudio]::mciSendString("record recsession", $sb, $sb.Capacity, [IntPtr]::Zero)
        }
    }
    
} catch {
    Write-Host "Recording failed: $_"
    # Try to save whatever we have
    try {
        [WinAudio]::mciSendString("stop recsession", $null, 0, [IntPtr]::Zero)
        [WinAudio]::mciSendString("save recsession `"$OutputFile`"", $null, 0, [IntPtr]::Zero)
        [WinAudio]::mciSendString("close recsession", $null, 0, [IntPtr]::Zero)
    } catch {
        Write-Host "Cleanup failed: $_"
    }
    exit 1
} finally {
    # Clean up temp files
    if (Test-Path $tempBat) {
        Remove-Item $tempBat -Force -ErrorAction SilentlyContinue
    }
}
