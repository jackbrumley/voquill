//go:build windows

package main

import (
	"fmt"
	"time"

	"golang.org/x/sys/windows"
)

var (
	user32               = windows.NewLazySystemDLL("user32.dll")
	procGetAsyncKeyState = user32.NewProc("GetAsyncKeyState")
	procGetKeyState      = user32.NewProc("GetKeyState")
)

const (
	VK_CONTROL  = 0x11 // Ctrl key
	VK_SHIFT    = 0x10 // Shift key
	VK_MENU     = 0x12 // Alt key
	KEY_PRESSED = 0x8000
)

// startHotkeyMonitoring starts monitoring for global hotkeys on Windows
func startHotkeyMonitoring() {
	fmt.Printf("Starting global hotkey monitoring for: %s\n", appState.config.Hotkey)
	go monitorWindowsHotkeys()
}

// monitorWindowsHotkeys continuously monitors for Ctrl+Shift+Alt key combination
func monitorWindowsHotkeys() {
	fmt.Println("Hotkey monitoring started for Ctrl+Shift+Alt")

	for {
		// Check if Ctrl+Shift+Alt are all pressed
		if isKeyPressed(VK_CONTROL) && isKeyPressed(VK_SHIFT) && isKeyPressed(VK_MENU) {
			if !appState.hotkeyPressed {
				fmt.Println("Hotkey combination detected - starting recording")
				appState.hotkeyPressed = true
				go recordAndTranscribeHotkey()
			}
		} else {
			// Keys are not pressed, stop recording if it was active
			if appState.hotkeyPressed {
				fmt.Println("Hotkey combination released - stopping recording")
				// Ensure minimum recording time before stopping
				go func() {
					// Wait at least 1 second before allowing stop
					time.Sleep(1 * time.Second)
					stopRecordingHotkey()
				}()
				appState.hotkeyPressed = false
			}
		}

		// Small sleep to prevent excessive CPU usage
		time.Sleep(50 * time.Millisecond)
	}
}

// isKeyPressed checks if a specific virtual key is currently pressed
func isKeyPressed(vkCode uintptr) bool {
	ret, _, _ := procGetAsyncKeyState.Call(vkCode)
	return (ret & KEY_PRESSED) != 0
}
