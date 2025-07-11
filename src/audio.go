package main

import (
	"encoding/binary"
	"os"
	"time"
	"github.com/gordonklaus/portaudio"
)

// recordWavInterruptible records audio and saves it as a WAV file with stop channel
func recordWavInterruptible(filename string, stopChan chan bool) error {
	portaudio.Initialize()
	defer portaudio.Terminate()

	// Use a dynamic slice to collect frames
	var allFrames []int16
	bufferSize := sampleRate / 10 // 100ms chunks
	
	stream, err := portaudio.OpenDefaultStream(1, 0, float64(sampleRate), bufferSize, func(in []int16) {
		allFrames = append(allFrames, in...)
	})
	if err != nil {
		return err
	}
	defer stream.Close()

	if err := stream.Start(); err != nil {
		return err
	}
	defer stream.Stop()

	// Record until stopped - NO TIMER, only stops when button clicked
	ticker := time.NewTicker(100 * time.Millisecond)
	defer ticker.Stop()

	for {
		select {
		case <-stopChan:
			// Stop recording immediately when button clicked
			goto writeFile
		case <-ticker.C:
			// Just continue recording - no time limit
			continue
		}
	}

writeFile:
	// Write the recorded audio to file
	f, err := os.Create(filename)
	if err != nil {
		return err
	}
	defer f.Close()

	// Write WAV file headers and data
	sampleSize := 2
	byteRate := sampleRate * sampleSize
	dataLen := len(allFrames) * sampleSize

	f.WriteString("RIFF")
	binary.Write(f, binary.LittleEndian, uint32(36+dataLen))
	f.WriteString("WAVEfmt ")
	binary.Write(f, binary.LittleEndian, uint32(16))
	binary.Write(f, binary.LittleEndian, uint16(1))
	binary.Write(f, binary.LittleEndian, uint16(1))
	binary.Write(f, binary.LittleEndian, uint32(sampleRate))
	binary.Write(f, binary.LittleEndian, uint32(byteRate))
	binary.Write(f, binary.LittleEndian, uint16(sampleSize))
	binary.Write(f, binary.LittleEndian, uint16(16))
	f.WriteString("data")
	binary.Write(f, binary.LittleEndian, uint32(dataLen))
	binary.Write(f, binary.LittleEndian, allFrames)
	return nil
}
