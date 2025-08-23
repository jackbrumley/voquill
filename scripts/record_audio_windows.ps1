# PowerShell script for Windows audio recording using Windows Media Control Interface (MCI)
param(
    [Parameter(Mandatory=$true)]
    [string]$OutputFile,
    
    [Parameter(Mandatory=$false)]
    [int]$MaxDuration = 30
)

# Add Windows Multimedia API support
Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
using System.Text;

public class AudioRecorder {
    [DllImport("winmm.dll")]
    public static extern int mciSendString(string command, StringBuilder returnValue, int returnLength, IntPtr winHandle);
}
"@

try {
    $sb = New-Object System.Text.StringBuilder(255)
    
    # Open new waveaudio device
    $result = [AudioRecorder]::mciSendString("open new type waveaudio alias myrecording", $sb, $sb.Capacity, [IntPtr]::Zero)
    if ($result -ne 0) {
        Write-Error "Failed to open audio device. Error code: $result"
        exit 1
    }
    
    # Set recording format to 16kHz, 16-bit, mono
    [AudioRecorder]::mciSendString("set myrecording time format ms", $sb, $sb.Capacity, [IntPtr]::Zero)
    [AudioRecorder]::mciSendString("set myrecording bitspersample 16", $sb, $sb.Capacity, [IntPtr]::Zero)
    [AudioRecorder]::mciSendString("set myrecording samplespersec 16000", $sb, $sb.Capacity, [IntPtr]::Zero)
    [AudioRecorder]::mciSendString("set myrecording channels 1", $sb, $sb.Capacity, [IntPtr]::Zero)
    
    # Start recording
    $result = [AudioRecorder]::mciSendString("record myrecording", $sb, $sb.Capacity, [IntPtr]::Zero)
    if ($result -ne 0) {
        Write-Error "Failed to start recording. Error code: $result"
        [AudioRecorder]::mciSendString("close myrecording", $sb, $sb.Capacity, [IntPtr]::Zero)
        exit 1
    }
    
    Write-Host "Recording started..."
    
    # Wait for recording to complete (will be interrupted by process termination)
    Start-Sleep -Seconds $MaxDuration
    
    # Save the recording
    $result = [AudioRecorder]::mciSendString("save myrecording `"$OutputFile`"", $sb, $sb.Capacity, [IntPtr]::Zero)
    if ($result -ne 0) {
        Write-Error "Failed to save recording. Error code: $result"
    } else {
        Write-Host "Recording saved to: $OutputFile"
    }
    
    # Close the recording device
    [AudioRecorder]::mciSendString("close myrecording", $sb, $sb.Capacity, [IntPtr]::Zero)
    
} catch {
    Write-Error "Recording failed: $_"
    # Ensure cleanup
    [AudioRecorder]::mciSendString("close myrecording", $sb, $sb.Capacity, [IntPtr]::Zero)
    exit 1
}
