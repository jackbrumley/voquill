package main

import (
	"fmt"
	"runtime"
	"time"
	"github.com/micmonay/keybd_event"
)

// simulateTyping simulates typing using HID-level keyboard events
func simulateTyping(text string) {
	// Create a new keyboard event
	kb, err := keybd_event.NewKeyBonding()
	if err != nil {
		fmt.Printf("Error creating keyboard binding: %v\n", err)
		return
	}
	
	// Type each character immediately - no startup delay
	for _, char := range text {
		// Convert character to key events
		err := typeCharacter(kb, char)
		if err != nil {
			fmt.Printf("Error typing character '%c': %v\n", char, err)
		}
		time.Sleep(appState.typingInterval)
	}
}

// typeCharacter types a single character using keyboard events
func typeCharacter(kb keybd_event.KeyBonding, char rune) error {
	var keyCode int
	var needShift bool
	
	// Map characters to key codes - fixed mappings
	switch char {
	// Letters (lowercase)
	case 'a': keyCode = keybd_event.VK_A
	case 'b': keyCode = keybd_event.VK_B
	case 'c': keyCode = keybd_event.VK_C
	case 'd': keyCode = keybd_event.VK_D
	case 'e': keyCode = keybd_event.VK_E
	case 'f': keyCode = keybd_event.VK_F
	case 'g': keyCode = keybd_event.VK_G
	case 'h': keyCode = keybd_event.VK_H
	case 'i': keyCode = keybd_event.VK_I
	case 'j': keyCode = keybd_event.VK_J
	case 'k': keyCode = keybd_event.VK_K
	case 'l': keyCode = keybd_event.VK_L
	case 'm': keyCode = keybd_event.VK_M
	case 'n': keyCode = keybd_event.VK_N
	case 'o': keyCode = keybd_event.VK_O
	case 'p': keyCode = keybd_event.VK_P
	case 'q': keyCode = keybd_event.VK_Q
	case 'r': keyCode = keybd_event.VK_R
	case 's': keyCode = keybd_event.VK_S
	case 't': keyCode = keybd_event.VK_T
	case 'u': keyCode = keybd_event.VK_U
	case 'v': keyCode = keybd_event.VK_V
	case 'w': keyCode = keybd_event.VK_W
	case 'x': keyCode = keybd_event.VK_X
	case 'y': keyCode = keybd_event.VK_Y
	case 'z': keyCode = keybd_event.VK_Z
	
	// Letters (uppercase)
	case 'A': keyCode = keybd_event.VK_A; needShift = true
	case 'B': keyCode = keybd_event.VK_B; needShift = true
	case 'C': keyCode = keybd_event.VK_C; needShift = true
	case 'D': keyCode = keybd_event.VK_D; needShift = true
	case 'E': keyCode = keybd_event.VK_E; needShift = true
	case 'F': keyCode = keybd_event.VK_F; needShift = true
	case 'G': keyCode = keybd_event.VK_G; needShift = true
	case 'H': keyCode = keybd_event.VK_H; needShift = true
	case 'I': keyCode = keybd_event.VK_I; needShift = true
	case 'J': keyCode = keybd_event.VK_J; needShift = true
	case 'K': keyCode = keybd_event.VK_K; needShift = true
	case 'L': keyCode = keybd_event.VK_L; needShift = true
	case 'M': keyCode = keybd_event.VK_M; needShift = true
	case 'N': keyCode = keybd_event.VK_N; needShift = true
	case 'O': keyCode = keybd_event.VK_O; needShift = true
	case 'P': keyCode = keybd_event.VK_P; needShift = true
	case 'Q': keyCode = keybd_event.VK_Q; needShift = true
	case 'R': keyCode = keybd_event.VK_R; needShift = true
	case 'S': keyCode = keybd_event.VK_S; needShift = true
	case 'T': keyCode = keybd_event.VK_T; needShift = true
	case 'U': keyCode = keybd_event.VK_U; needShift = true
	case 'V': keyCode = keybd_event.VK_V; needShift = true
	case 'W': keyCode = keybd_event.VK_W; needShift = true
	case 'X': keyCode = keybd_event.VK_X; needShift = true
	case 'Y': keyCode = keybd_event.VK_Y; needShift = true
	case 'Z': keyCode = keybd_event.VK_Z; needShift = true
	
	// Numbers
	case '0': keyCode = keybd_event.VK_0
	case '1': keyCode = keybd_event.VK_1
	case '2': keyCode = keybd_event.VK_2
	case '3': keyCode = keybd_event.VK_3
	case '4': keyCode = keybd_event.VK_4
	case '5': keyCode = keybd_event.VK_5
	case '6': keyCode = keybd_event.VK_6
	case '7': keyCode = keybd_event.VK_7
	case '8': keyCode = keybd_event.VK_8
	case '9': keyCode = keybd_event.VK_9
	
	// Common punctuation
	case ' ': keyCode = keybd_event.VK_SPACE
	case '.': keyCode = keybd_event.VK_DOT
	case ',': keyCode = keybd_event.VK_COMMA
	case '!': keyCode = keybd_event.VK_1; needShift = true
	case '?': keyCode = keybd_event.VK_SLASH; needShift = true
	case ':': keyCode = keybd_event.VK_SEMICOLON; needShift = true
	case ';': keyCode = keybd_event.VK_SEMICOLON
	case '-': keyCode = keybd_event.VK_MINUS
	case '_': keyCode = keybd_event.VK_MINUS; needShift = true
	case '\n': keyCode = keybd_event.VK_ENTER
	
	default:
		// For unsupported characters, skip them
		return nil
	}
	
	// Set the key
	kb.SetKeys(keyCode)
	
	// Add shift if needed
	if needShift {
		kb.HasSHIFT(true)
	} else {
		kb.HasSHIFT(false)
	}
	
	// Press and release the key
	err := kb.Launching()
	if err != nil {
		return err
	}
	
	return nil
}
