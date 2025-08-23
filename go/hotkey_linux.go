//go:build linux

package main

import (
	"bytes"
	"encoding/binary"
	"fmt"
	"os"
	"path/filepath"
	"syscall"
	"unsafe"
)

const (
	EV_KEY = 0x01
	KEY_LEFTCTRL  = 29
	KEY_RIGHTCTRL = 97
	KEY_LEFTSHIFT = 42
	KEY_RIGHTSHIFT = 54
	KEY_LEFTALT = 56
	KEY_RIGHTALT = 100
)

type InputEvent struct {
	Time  syscall.Timeval
	Type  uint16
	Code  uint16
	Value int32
}

// startHotkeyMonitoring starts monitoring for global hotkeys on Linux
func startHotkeyMonitoring() {
	fmt.Printf("Starting global hotkey monitoring for: %s\n", appState.config.Hotkey)
	
	// Monitor keyboard events using /dev/input/event* files
	go monitorLinuxHotkeys()
}

// monitorLinuxHotkeys monitors for Ctrl+Shift+Alt using /dev/input/event* files
func monitorLinuxHotkeys() {
	// Find keyboard event files
	eventFilenames, err := filepath.Glob("/dev/input/event*")
	if err != nil {
		fmt.Printf("Error finding input event files: %v\n", err)
		return
	}
	
	if len(eventFilenames) == 0 {
		fmt.Println("No input event files found, hotkey monitoring disabled")
		return
	}
	
	// Try to open all keyboard event files and monitor them
	var eventFiles []*os.File
	for _, filename := range eventFilenames {
		file, err := os.Open(filename)
		if err == nil {
			fmt.Printf("Opened input device: %s\n", filename)
			eventFiles = append(eventFiles, file)
		}
	}
	
	if len(eventFiles) == 0 {
		fmt.Println("Could not open any input event files. Try running with sudo or add user to input group.")
		return
	}
	
	// Monitor all devices in parallel
	for i, file := range eventFiles {
		go func(deviceFile *os.File, deviceIndex int) {
			defer deviceFile.Close()
			monitorDevice(deviceFile, deviceIndex)
		}(file, i)
	}
	
	// Keep the main goroutine alive
	select {}
}

// monitorDevice monitors a single input device for keyboard events
func monitorDevice(eventFile *os.File, deviceIndex int) {
	
	// Track key states with better logic
	ctrlPressed := false
	shiftPressed := false
	altPressed := false
	
	// Read input events
	eventSize := int(unsafe.Sizeof(InputEvent{}))
	buffer := make([]byte, eventSize)
	
	for {
		n, err := eventFile.Read(buffer)
		if err != nil {
			fmt.Printf("Error reading input events: %v\n", err)
			break
		}
		
		if n != eventSize {
			continue
		}
		
		// Parse the input event
		var event InputEvent
		err = binary.Read(bytes.NewReader(buffer), binary.LittleEndian, &event)
		if err != nil {
			continue
		}
		
		// Only process key events (ignore mouse events)
		if event.Type != EV_KEY {
			continue
		}
		
		isPressed := event.Value == 1
		isReleased := event.Value == 0
		
		// Track modifier keys
		switch event.Code {
		case KEY_LEFTCTRL, KEY_RIGHTCTRL:
			if isPressed {
				ctrlPressed = true
			} else if isReleased {
				ctrlPressed = false
			}
		case KEY_LEFTSHIFT, KEY_RIGHTSHIFT:
			if isPressed {
				shiftPressed = true
			} else if isReleased {
				shiftPressed = false
			}
		case KEY_LEFTALT, KEY_RIGHTALT:
			if isPressed {
				altPressed = true
			} else if isReleased {
				altPressed = false
			}
		}
		
		// Check hotkey combination on any key state change
		hotkeyActive := ctrlPressed && shiftPressed && altPressed
		
		if hotkeyActive && !appState.hotkeyPressed {
			fmt.Println("Hotkey pressed - starting recording")
			appState.hotkeyPressed = true
			go recordAndTranscribeHotkey()
		} else if !hotkeyActive && appState.hotkeyPressed {
			fmt.Println("Hotkey released - stopping recording")
			appState.hotkeyPressed = false
			stopRecordingHotkey()
		}
	}
}
