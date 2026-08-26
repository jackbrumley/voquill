"""
Acoustic Audio DSP & Voice Modulation Effects for Voquill Voice Macros.
Transforms standard TTS voices into authentic Titan Mech, Tactical Radio,
Cyberpunk Ship EVA, and Robotic audio profiles.
"""

from __future__ import annotations

import logging
import numpy as np
from scipy import signal

logger = logging.getLogger("voquill.tts.dsp")


def pitch_shift(samples: np.ndarray, sample_rate: int, semitones: float) -> np.ndarray:
    """
    Shifts pitch by the specified number of semitones while maintaining approximate tempo.
    Uses time-domain pitch shifting with overlap-add (TD-PSOLA) approximation.
    """
    if abs(semitones) < 0.1:
        return samples

    factor = 2.0 ** (semitones / 12.0)
    # Target length after pitch resample
    num_samples = len(samples)
    resampled_len = int(round(num_samples / factor))
    if resampled_len <= 0:
        return samples

    # Resample to change pitch
    shifted = signal.resample(samples, resampled_len)

    # Stretch/compress back to original duration using linear interpolation
    orig_indices = np.linspace(0, resampled_len - 1, num_samples)
    out = np.interp(orig_indices, np.arange(resampled_len), shifted)
    return out.astype(np.float32)


def ring_modulation(samples: np.ndarray, sample_rate: int, freq_hz: float = 45.0, mix: float = 0.5) -> np.ndarray:
    """
    Ring modulation: multiplies the audio signal by a low-frequency sine carrier
    to create the signature mechanical / metallic Dalek / MechWarrior / Titanfall robotic growl.
    """
    if mix <= 0.0:
        return samples

    t = np.arange(len(samples)) / float(sample_rate)
    carrier = np.sin(2.0 * np.pi * freq_hz * t)
    modulated = samples * carrier
    return ((1.0 - mix) * samples + mix * modulated).astype(np.float32)


def radio_bandpass_filter(samples: np.ndarray, sample_rate: int, low_hz: float = 350.0, high_hz: float = 3400.0) -> np.ndarray:
    """
    Simulates analog walkie-talkie / military comms microphone response using
    a Butterworth bandpass filter.
    """
    nyquist = 0.5 * sample_rate
    low = max(20.0, low_hz) / nyquist
    high = min(nyquist - 20.0, high_hz) / nyquist

    if low >= high or high >= 1.0:
        return samples

    sos = signal.butter(4, [low, high], btype='bandpass', output='sos')
    filtered = signal.sosfilt(sos, samples)
    return filtered.astype(np.float32)


def soft_saturation(samples: np.ndarray, drive: float = 1.5) -> np.ndarray:
    """
    Applies gentle analog tape / radio tube saturation to give audio grit and presence.
    """
    return (np.tanh(samples * drive) / np.tanh(drive)).astype(np.float32)


def apply_mech_effect(samples: np.ndarray, sample_rate: int, pitch_semitones: float = -5.0) -> np.ndarray:
    """
    🤖 Titan Mech: Deep commanding pitch, sub-bass harmonics, and metallic mechanical ring modulation.
    """
    # 1. Pitch shift down
    out = pitch_shift(samples, sample_rate, pitch_semitones)

    # 2. Ring modulation for metallic robot texture (48Hz carrier, 38% blend)
    out = ring_modulation(out, sample_rate, freq_hz=48.0, mix=0.38)

    # 3. Low-shelf bass boost for chassis weight (boost < 200Hz)
    nyquist = 0.5 * sample_rate
    cutoff = min(220.0 / nyquist, 0.45)
    sos = signal.butter(2, cutoff, btype='lowpass', output='sos')
    bass = signal.sosfilt(sos, out)
    out = out + 0.45 * bass

    # 4. Soft saturation
    out = soft_saturation(out, drive=1.4)

    # Normalize peak to 0.95
    peak = np.max(np.abs(out))
    if peak > 1e-4:
        out = (out / peak) * 0.95

    return out.astype(np.float32)


def apply_radio_effect(samples: np.ndarray, sample_rate: int, pitch_semitones: float = 0.0) -> np.ndarray:
    """
    📻 Tactical Radio: 350Hz-3.4kHz military bandpass filter with transmitter overdrive.
    """
    out = samples
    if abs(pitch_semitones) > 0.1:
        out = pitch_shift(out, sample_rate, pitch_semitones)

    # 1. Bandpass filter
    out = radio_bandpass_filter(out, sample_rate, low_hz=350.0, high_hz=3200.0)

    # 2. Add subtle radio grit / saturation
    out = soft_saturation(out, drive=2.0)

    # 3. Slight ring modulation for radio carrier buzz (120Hz carrier, 10% blend)
    out = ring_modulation(out, sample_rate, freq_hz=120.0, mix=0.10)

    # Normalize peak
    peak = np.max(np.abs(out))
    if peak > 1e-4:
        out = (out / peak) * 0.92

    return out.astype(np.float32)


def apply_eva_effect(samples: np.ndarray, sample_rate: int, pitch_semitones: float = 1.0) -> np.ndarray:
    """
    🛸 Cyberpunk EVA: Futuristic ship computer with crisp presence and metallic resonance.
    """
    out = samples
    if abs(pitch_semitones) > 0.1:
        out = pitch_shift(out, sample_rate, pitch_semitones)

    # 1. Subtle ring modulation (80Hz, 15% mix)
    out = ring_modulation(out, sample_rate, freq_hz=80.0, mix=0.15)

    # 2. High-pass filter (>160Hz) to remove muddy low-end
    nyquist = 0.5 * sample_rate
    cutoff = 160.0 / nyquist
    if cutoff < 0.45:
        sos = signal.butter(2, cutoff, btype='highpass', output='sos')
        out = signal.sosfilt(sos, out)

    # 3. Add short early reflection (18ms delay, 0.25 feedback)
    delay_samples = int(0.018 * sample_rate)
    if delay_samples < len(out):
        echo = np.zeros_like(out)
        echo[delay_samples:] = out[:-delay_samples] * 0.25
        out = out + echo

    # Normalize peak
    peak = np.max(np.abs(out))
    if peak > 1e-4:
        out = (out / peak) * 0.95

    return out.astype(np.float32)


def apply_dsp_effect(
    samples: np.ndarray,
    sample_rate: int,
    effect: str = "clean",
    pitch: float = 0.0,
) -> np.ndarray:
    """
    Routes audio through the selected DSP effect profile.
    """
    effect_norm = (effect or "clean").lower().strip()

    if effect_norm in ("mech", "titan_mech", "robot", "titan"):
        # Default pitch for mech is -5 semitones if pitch not specified (0.0)
        actual_pitch = pitch if abs(pitch) > 0.01 else -5.0
        return apply_mech_effect(samples, sample_rate, pitch_semitones=actual_pitch)

    elif effect_norm in ("radio", "tactical_radio", "comms"):
        return apply_radio_effect(samples, sample_rate, pitch_semitones=pitch)

    elif effect_norm in ("eva", "cyberpunk_eva", "ship_ai", "spaceship"):
        return apply_eva_effect(samples, sample_rate, pitch_semitones=pitch)

    else:
        # Clean / natural
        if abs(pitch) > 0.01:
            shifted = pitch_shift(samples, sample_rate, pitch)
            peak = np.max(np.abs(shifted))
            if peak > 1e-4:
                shifted = (shifted / peak) * 0.95
            return shifted
        return samples
