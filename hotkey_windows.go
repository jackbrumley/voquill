//go:build windows

package main

import (
	"fmt"
	"time"
	"unsafe"

	"golang.org/x/sys/windows"
)

var (
	user32               = windows.NewLazySystemDLL("user32.dll")
	procRegisterHotKey   = user32.NewProc("RegisterHotKey")
	procUnregisterHotKey = user32.NewProc("UnregisterHotKey")
	procPeekMessage      = user32.NewProc("PeekMessageW")
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
		0,                     // hWnd (NULL for current thread)
		1,                     // id (unique identifier)
		MOD_CONTROL|MOD_SHIFT, // fsModifiers
		VK_R,                  // vk (virtual key code for 'R')
	)

	if ret == 0 {
		fmt.Printf("Failed to register hotkey: %v\n", err)
		return
	}

	fmt.Println("Hotkey Ctrl+Shift+R registered successfully")

	// Non-blocking message loop to listen for hotkey events
	var msg MSG
	for {
		// Use PeekMessage instead of GetMessage to avoid blocking
		ret, _, _ := procPeekMessage.Call(
			uintptr(unsafe.Pointer(&msg)),
			0, // hWnd (NULL for any window)
			0, // wMsgFilterMin
			0, // wMsgFilterMax
			1, // PM_REMOVE - remove message from queue
		)

		if ret != 0 { // Message available
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

		// Small sleep to prevent CPU spinning and allow other operations
		time.Sleep(10 * time.Millisecond)
	}

	// Cleanup: unregister the hotkey
	procUnregisterHotKey.Call(0, 1)
}
