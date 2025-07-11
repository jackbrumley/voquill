//go:build windows

package main

import (
	"fmt"
	"time"
	"unsafe"

	"golang.org/x/sys/windows"
)

var (
	user32                = windows.NewLazySystemDLL("user32.dll")
	procRegisterHotKey    = user32.NewProc("RegisterHotKey")
	procUnregisterHotKey  = user32.NewProc("UnregisterHotKey")
	procGetMessage        = user32.NewProc("GetMessageW")
)

const (
	MOD_CONTROL = 0x0002
	MOD_SHIFT   = 0x0004
	VK_R        = 0x52
	WM_HOTKEY   = 0x0312
)

type MSG struct {
	Hwnd    uintptr
	Message uint32
	WParam  uintptr
	LParam  uintptr
	Time    uint32
	Pt      struct{ X, Y int32 }
}

// startHotkeyMonitoring starts monitoring for global hotkeys on Windows
func startHotkeyMonitoring() {
	fmt.Printf("Starting global hotkey monitoring for: %s\n", appState.config.Hotkey)
	
	// Register Ctrl+Shift+R hotkey
	go monitorWindowsHotkeys()
}

// monitorWindowsHotkeys monitors for Ctrl+Shift+R using Windows API
func monitorWindowsHotkeys() {
	// Register the hotkey (Ctrl+Shift+R)
	ret, _, err := procRegisterHotKey.Call(
		0,                           // hWnd (NULL for current thread)
		1,                           // id (unique identifier)
		MOD_CONTROL|MOD_SHIFT,       // fsModifiers
		VK_R,                        // vk (virtual key code for 'R')
	)
	
	if ret == 0 {
		fmt.Printf("Failed to register hotkey: %v\n", err)
		return
	}
	
	fmt.Println("Hotkey Ctrl+Shift+R registered successfully")
	
	// Message loop to listen for hotkey events
	var msg MSG
	for {
		ret, _, _ := procGetMessage.Call(
			uintptr(unsafe.Pointer(&msg)),
			0, // hWnd (NULL for any window)
			0, // wMsgFilterMin
			0, // wMsgFilterMax
		)
		
		if ret == 0 { // WM_QUIT
			break
		}
		
		if msg.Message == WM_HOTKEY {
			if msg.WParam == 1 { // Our hotkey ID
				if !appState.hotkeyPressed {
					fmt.Println("Hotkey pressed - starting recording")
					appState.hotkeyPressed = true
					go recordAndTranscribeHotkey()
					
					// Wait for key release (simplified - in real implementation you'd monitor key up events)
					go func() {
						// Simple timeout-based approach for now
						// In a full implementation, you'd monitor for key release events
						for appState.isRecording {
							time.Sleep(100 * time.Millisecond)
						}
						if appState.hotkeyPressed {
							fmt.Println("Recording finished - hotkey released")
							appState.hotkeyPressed = false
						}
					}()
				}
			}
		}
	}
	
	// Cleanup: unregister the hotkey
	procUnregisterHotKey.Call(0, 1)
}
