"""
Acoustic Audio DSP & Voice Modulation Effects for Voquill Voice Macros.
Transforms standard TTS voices into authentic Titan Mech, Tactical Radio,
Cyberpunk Ship EVA, Flight Deck ATC, and Studio audio profiles.
"""

from __future__ import annotations

import logging
import numpy as np
from scipy import signal

logger = logging.getLogger("voquill.tts.dsp")


def to_stereo(samples: np.ndarray) -> np.ndarray:
    """Converts mono audio array to balanced 2-channel stereo (L + R identical)."""
    if samples.ndim == 1:
        return np.column_stack((samples, samples)).astype(np.float32)
    return samples.astype(np.float32)


def _synthesize_chime(chime_type: str, sample_rate: int) -> np.ndarray:
    """Core synthesizer for all tactical chirps, squelches, clicks, and cockpit tones."""
    sr = float(sample_rate)
    ctype = (chime_type or "none").lower().strip()

    if ctype in ("tactical_double_beep", "cs_double_beep", "double_beep"):
        # Counter-Strike / Tactical Double Beep (880Hz -> 1175Hz)
        t1 = np.arange(int(0.028 * sr)) / sr
        tone1 = np.sin(2.0 * np.pi * 880.0 * t1) * 0.18
        tone1 *= np.sin(np.linspace(0, np.pi, len(tone1)))

        gap = np.zeros(int(0.012 * sr), dtype=np.float32)

        t2 = np.arange(int(0.032 * sr)) / sr
        tone2 = np.sin(2.0 * np.pi * 1175.0 * t2) * 0.20
        tone2 *= np.sin(np.linspace(0, np.pi, len(tone2)))

        return np.concatenate([tone1, gap, tone2]).astype(np.float32)

    elif ctype in ("radio_click", "mic_click", "click"):
        # 16ms subtle mic key-click pop
        n_samples = int(0.016 * sr)
        t = np.arange(n_samples) / sr
        click_pop = np.sin(2.0 * np.pi * 300.0 * t) * 0.14
        noise = np.random.normal(0, 0.10, n_samples)
        fade = np.linspace(1.0, 0.0, n_samples) ** 2
        return ((click_pop + noise) * fade).astype(np.float32)

    elif ctype in ("cockpit_chime", "chime", "sci_fi_chime"):
        # Sci-Fi 3-tone chord (587Hz + 880Hz + 1320Hz)
        n_samples = int(0.075 * sr)
        t = np.arange(n_samples) / sr
        decay = np.exp(-t * 32.0)
        chime = (
            0.5 * np.sin(2 * np.pi * 587 * t)
            + 0.3 * np.sin(2 * np.pi * 880 * t)
            + 0.2 * np.sin(2 * np.pi * 1320 * t)
        ) * 0.18 * decay
        return chime.astype(np.float32)

    elif ctype in ("transmit_blip", "blip"):
        # 18ms high-tech transmit blip (1760Hz)
        n_samples = int(0.018 * sr)
        t = np.arange(n_samples) / sr
        blip = np.sin(2.0 * np.pi * 1760.0 * t) * 0.15
        blip *= np.sin(np.linspace(0, np.pi, n_samples))
        return blip.astype(np.float32)

    elif ctype in ("radio_squelch", "squelch"):
        # 35ms analog radio squelch chirp
        squelch_len = int(0.035 * sr)
        squelch_noise = np.random.normal(0, 0.18, squelch_len)
        fade = np.linspace(1.0, 0.0, squelch_len) ** 2
        squelch_chirp = (
            np.sin(2.0 * np.pi * 2400.0 * np.arange(squelch_len) / sr) * 0.08
        )
        return ((squelch_noise + squelch_chirp) * fade).astype(np.float32)

    elif ctype in ("cs_radio_off", "cs_squelch", "tactical_squelch"):
        # Counter-Strike style comms release burst (42ms)
        n_samples = int(0.042 * sr)
        t = np.arange(n_samples) / sr
        noise = np.random.normal(0, 0.22, n_samples)
        sweep_freq = np.linspace(1900.0, 450.0, n_samples)
        chirp = np.sin(2.0 * np.pi * sweep_freq * t) * 0.14
        fade = np.linspace(1.0, 0.0, n_samples) ** 1.5
        return ((noise + chirp) * fade).astype(np.float32)

    elif ctype in ("mic_release_click", "release_click", "unkey"):
        # Sharp 18ms mic un-key pop
        n_samples = int(0.018 * sr)
        t = np.arange(n_samples) / sr
        click_pop = np.sin(2.0 * np.pi * 220.0 * t) * 0.18
        fade = np.linspace(1.0, 0.0, n_samples) ** 3
        return (click_pop * fade).astype(np.float32)

    elif ctype in ("cockpit_ack", "ack"):
        # Two-tone acknowledgment (1046Hz -> 1318Hz, 24ms each)
        n1 = int(0.024 * sr)
        t1 = np.arange(n1) / sr
        tone1 = np.sin(2.0 * np.pi * 1046.0 * t1) * 0.15 * np.sin(np.linspace(0, np.pi, n1))

        n2 = int(0.024 * sr)
        t2 = np.arange(n2) / sr
        tone2 = np.sin(2.0 * np.pi * 1318.0 * t2) * 0.15 * np.sin(np.linspace(0, np.pi, n2))

        return np.concatenate([tone1, tone2]).astype(np.float32)

    return np.zeros(0, dtype=np.float32)


def generate_opening_chime(chime_type: str, sample_rate: int) -> np.ndarray:
    """Generates synthetic opening radio chirps, beeps, and cockpit chimes with a clean pre-voice pause."""
    raw = _synthesize_chime(chime_type, sample_rate)
    if len(raw) == 0:
        return np.zeros(0, dtype=np.float32)
    sr = float(sample_rate)
    pause = np.zeros(int(0.075 * sr), dtype=np.float32)
    return np.concatenate([raw, pause]).astype(np.float32)


def generate_closing_chime(chime_type: str, sample_rate: int) -> np.ndarray:
    """Generates synthetic closing radio squelches, release clicks, and ack tones with a clean post-voice gap."""
    raw = _synthesize_chime(chime_type, sample_rate)
    if len(raw) == 0:
        return np.zeros(0, dtype=np.float32)
    sr = float(sample_rate)
    gap = np.zeros(int(0.020 * sr), dtype=np.float32)
    return np.concatenate([gap, raw]).astype(np.float32)


def pitch_shift(samples: np.ndarray, sample_rate: int, semitones: float) -> np.ndarray:
    """
    Shifts pitch by the specified number of semitones while maintaining exact duration
    using Phase Vocoder STFT processing.
    Negative semitones -> lower/deeper pitch.
    Positive semitones -> higher/brighter pitch.
    """
    if abs(semitones) < 0.05 or len(samples) < 512:
        return samples

    factor = 2.0 ** (semitones / 12.0)
    n_fft = 2048
    hop_length = n_fft // 4

    _, _, zxx = signal.stft(
        samples,
        fs=sample_rate,
        nperseg=n_fft,
        noverlap=n_fft - hop_length,
        boundary="zeros",
    )

    step = 1.0 / factor
    time_steps = np.arange(0, zxx.shape[1] - 1, step)
    if len(time_steps) == 0:
        return samples

    new_zxx = np.zeros((zxx.shape[0], len(time_steps)), dtype=complex)
    phase_acc = np.angle(zxx[:, 0])
    omega = 2.0 * np.pi * hop_length * np.arange(zxx.shape[0]) / n_fft

    for i, t in enumerate(time_steps):
        t_int = int(np.floor(t))
        t_frac = t - t_int

        frame1 = zxx[:, t_int]
        frame2 = zxx[:, min(t_int + 1, zxx.shape[1] - 1)]

        mag = (1.0 - t_frac) * np.abs(frame1) + t_frac * np.abs(frame2)

        delta_phase = np.angle(frame2) - np.angle(frame1) - omega * step
        delta_phase = delta_phase - 2.0 * np.pi * np.round(delta_phase / (2.0 * np.pi))

        true_freq = omega + delta_phase / step
        phase_acc = phase_acc + true_freq

        new_zxx[:, i] = mag * np.exp(1j * phase_acc)

    _, stretched = signal.istft(
        new_zxx,
        fs=sample_rate,
        nperseg=n_fft,
        noverlap=n_fft - hop_length,
        boundary=True,
    )

    orig_len = len(samples)
    if len(stretched) == 0:
        return samples

    resampled = signal.resample(stretched, orig_len)
    return resampled.astype(np.float32)


def comb_filter(
    samples: np.ndarray,
    sample_rate: int,
    delay_ms: float,
    feedback: float,
) -> np.ndarray:
    """Gain-normalized IIR Feedback Comb Filter for metallic chassis resonance."""
    delay_samples = int(round((delay_ms / 1000.0) * sample_rate))
    if delay_samples <= 0 or delay_samples >= len(samples):
        return samples

    feedback = max(-0.85, min(0.85, feedback))
    a = np.zeros(delay_samples + 1, dtype=np.float32)
    a[0] = 1.0
    a[delay_samples] = -feedback
    b = np.array([1.0 - abs(feedback) * 0.4], dtype=np.float32)

    return signal.lfilter(b, a, samples).astype(np.float32)


def apply_flanger(
    samples: np.ndarray,
    sample_rate: int,
    rate_hz: float = 0.5,
    depth_ms: float = 2.0,
    base_delay_ms: float = 3.0,
    mix: float = 0.25,
) -> np.ndarray:
    """Modulated delay for robotic / chorus metallic timbre."""
    t = np.arange(len(samples)) / float(sample_rate)
    delay_modulation = (
        (base_delay_ms + depth_ms * np.sin(2.0 * np.pi * rate_hz * t))
        * 0.001
        * sample_rate
    )

    indices = np.arange(len(samples)) - delay_modulation
    indices = np.clip(indices, 0, len(samples) - 1)

    idx_floor = indices.astype(int)
    idx_ceil = np.clip(idx_floor + 1, 0, len(samples) - 1)
    frac = indices - idx_floor

    delayed = (1.0 - frac) * samples[idx_floor] + frac * samples[idx_ceil]
    return ((1.0 - mix) * samples + mix * delayed).astype(np.float32)


def apply_mech_effect(
    samples: np.ndarray,
    sample_rate: int,
    pitch_semitones: float = -4.0,
) -> np.ndarray:
    """
    🤖 Titan Mech: Heavy armored cockpit AI with metallic chassis comb resonance,
    sub-bass harmonic weight, and iron saturation.
    """
    shifted = pitch_shift(samples, sample_rate, pitch_semitones)

    nyquist = 0.5 * sample_rate
    cutoff = min(160.0 / nyquist, 0.45)
    sos_low = signal.butter(2, cutoff, btype="lowpass", output="sos")
    sub_bass = signal.sosfilt(sos_low, shifted)
    sub_bass_saturated = np.tanh(sub_bass * 2.2)

    comb1 = comb_filter(shifted, sample_rate, delay_ms=13.5, feedback=-0.32)
    comb2 = comb_filter(shifted, sample_rate, delay_ms=21.0, feedback=-0.25)
    metallic = 0.5 * comb1 + 0.5 * comb2

    flanged = apply_flanger(
        metallic,
        sample_rate,
        rate_hz=0.6,
        depth_ms=1.8,
        base_delay_ms=3.0,
        mix=0.25,
    )

    combined = 0.60 * flanged + 0.35 * sub_bass_saturated + 0.20 * shifted
    out = np.tanh(combined * 1.5) / np.tanh(1.5)

    # DC block highpass at 25Hz
    sos_dc = signal.butter(1, min(25.0 / nyquist, 0.4), btype="highpass", output="sos")
    out = signal.sosfilt(sos_dc, out)

    peak = np.max(np.abs(out))
    if peak > 1e-4:
        out = (out / peak) * 0.95

    return out.astype(np.float32)


def apply_radio_effect(
    samples: np.ndarray,
    sample_rate: int,
    pitch_semitones: float = 0.0,
    opening_chime: str = "none",
    closing_chime: str = "radio_squelch",
) -> np.ndarray:
    """
    📻 Tactical Comms: Military VHF/UHF bandpass, mic diaphragm overdrive,
    dynamic RF carrier noise, and tail squelch tone burst.
    """
    out = samples
    if abs(pitch_semitones) > 0.1:
        out = pitch_shift(out, sample_rate, pitch_semitones)

    # 1. Strict Military Bandpass (450Hz to 3.2kHz)
    nyquist = 0.5 * sample_rate
    low = 450.0 / nyquist
    high = min(3200.0 / nyquist, 0.95)
    if low < high:
        sos = signal.butter(4, [low, high], btype="bandpass", output="sos")
        out = signal.sosfilt(sos, out)

    # 2. Non-linear asymmetrical overdrive
    out = np.tanh(out * 2.2) + 0.08 * (out**2)

    # 3. Dynamic RF static noise floor during transmission
    np.random.seed(42)
    noise = np.random.normal(0, 0.015, len(out))
    if low < high:
        noise = signal.sosfilt(sos, noise)

    env = np.abs(out)
    sos_env = signal.butter(
        1, min(10.0 / nyquist, 0.4), btype="lowpass", output="sos"
    )
    env_smooth = signal.sosfilt(sos_env, env)
    env_smooth = np.clip(env_smooth / (np.max(env_smooth) + 1e-5), 0.15, 1.0)
    out = out + noise * env_smooth * 0.35

    # 4. Attach opening and closing chimes
    open_tone = generate_opening_chime(opening_chime, sample_rate)
    close_tone = generate_closing_chime(closing_chime, sample_rate)

    parts = []
    if len(open_tone) > 0:
        parts.append(open_tone)
    parts.append(out)
    if len(close_tone) > 0:
        parts.append(close_tone)

    out = np.concatenate(parts)

    peak = np.max(np.abs(out))
    if peak > 1e-4:
        out = (out / peak) * 0.68

    return out.astype(np.float32)


def apply_eva_effect(
    samples: np.ndarray,
    sample_rate: int,
    pitch_semitones: float = 1.0,
) -> np.ndarray:
    """
    🛸 Cyberpunk EVA: Futuristic spaceship bridge computer with multi-tap
    hologram early reflections and air clarity polish.
    """
    out = samples
    if abs(pitch_semitones) > 0.1:
        out = pitch_shift(out, sample_rate, pitch_semitones)

    nyquist = 0.5 * sample_rate
    cutoff = min(180.0 / nyquist, 0.45)
    sos_hp = signal.butter(2, cutoff, btype="highpass", output="sos")
    out = signal.sosfilt(sos_hp, out)

    taps = [(0.014, 0.28), (0.028, 0.18), (0.048, 0.10)]
    reflections = np.zeros_like(out)
    for delay_sec, gain in taps:
        d_samples = int(delay_sec * sample_rate)
        if d_samples < len(out):
            reflections[d_samples:] += out[:-d_samples] * gain

    out = out + reflections

    shelf_cutoff = min(4500.0 / nyquist, 0.85)
    sos_air = signal.butter(1, shelf_cutoff, btype="highpass", output="sos")
    air = signal.sosfilt(sos_air, out)
    out = out + 0.35 * air

    out = apply_flanger(
        out,
        sample_rate,
        rate_hz=0.4,
        depth_ms=1.2,
        base_delay_ms=2.0,
        mix=0.15,
    )

    peak = np.max(np.abs(out))
    if peak > 1e-4:
        out = (out / peak) * 0.68

    return out.astype(np.float32)


def apply_flight_deck_effect(
    samples: np.ndarray,
    sample_rate: int,
    pitch_semitones: float = 0.0,
) -> np.ndarray:
    """
    ✈️ Flight Deck ATC: Aviation cockpit intercom clarity with cabin compression.
    """
    out = samples
    if abs(pitch_semitones) > 0.1:
        out = pitch_shift(out, sample_rate, pitch_semitones)

    nyquist = 0.5 * sample_rate
    low = 320.0 / nyquist
    high = min(4600.0 / nyquist, 0.92)
    if low < high:
        sos = signal.butter(3, [low, high], btype="bandpass", output="sos")
        out = signal.sosfilt(sos, out)

    out = np.tanh(out * 1.3) / np.tanh(1.3)

    peak = np.max(np.abs(out))
    if peak > 1e-4:
        out = (out / peak) * 0.68

    return out.astype(np.float32)


def apply_custom_dsp(
    samples: np.ndarray,
    sample_rate: int,
    pitch: float = 0.0,
    sub_bass: float = 0.0,
    comb_mix: float = 0.0,
    flanger_mix: float = 0.0,
    radio_bandpass: bool = False,
    radio_drive: float = 1.0,
    rf_noise: float = 0.0,
    opening_chime: str = "none",
    closing_chime: str = "none",
) -> np.ndarray:
    """
    Flexible DSP pipeline for the Voquill Voice Lab interactive tuner.
    """
    out = samples
    if abs(pitch) > 0.05:
        out = pitch_shift(out, sample_rate, pitch)

    nyquist = 0.5 * sample_rate

    # 1. Radio Bandpass & Overdrive
    if radio_bandpass:
        low = 420.0 / nyquist
        high = min(3400.0 / nyquist, 0.95)
        sos = signal.butter(4, [low, high], btype="bandpass", output="sos")
        out = signal.sosfilt(sos, out)

        # Drive / Saturation
        if radio_drive > 1.01:
            out = np.tanh(out * radio_drive) / np.tanh(radio_drive)

        # RF Noise
        if rf_noise > 0.01:
            np.random.seed(42)
            noise = np.random.normal(0, 0.03, len(out))
            env = np.abs(out)
            sos_env = signal.butter(1, min(12.0 / nyquist, 0.4), btype="lowpass", output="sos")
            env_smooth = signal.sosfilt(sos_env, env)
            env_smooth = np.clip(env_smooth / (np.max(env_smooth) + 1e-5), 0.15, 1.0)
            out = out + noise * env_smooth * rf_noise

    # 3. Sub-Bass & Heavy Body Punch (0% flat -> 100% massive +18dB chest/subwoofer boom)
    if sub_bass > 0.01:
        # Lowpass filter below 260Hz captures vocal chest fundamental
        cutoff = min(260.0 / nyquist, 0.45)
        sos_sub = signal.butter(2, cutoff, btype="lowpass", output="sos")
        low_body = signal.sosfilt(sos_sub, out)

        # High-frequency damping when sub-bass is high to create dark, cinematic vocal mass
        sos_damp = signal.butter(1, min(2200.0 / nyquist, 0.85), btype="lowpass", output="sos")
        damped_body = signal.sosfilt(sos_damp, out)

        # Peaking sub-thump at 90Hz
        sos_thump = signal.butter(2, [min(60.0 / nyquist, 0.4), min(130.0 / nyquist, 0.45)], btype="bandpass", output="sos")
        thump = signal.sosfilt(sos_thump, out)

        # Heavy saturated sub-harmonics (gain scales dynamically up to +18dB)
        sub_gain = 1.0 + sub_bass * 4.5
        sub_sat = np.tanh(low_body * 2.4) * sub_gain

        out = (1.0 - sub_bass * 0.35) * damped_body + sub_sat * sub_bass + thump * (sub_bass * 2.2)

        # DC blocking highpass filter at 25Hz
        sos_dc = signal.butter(1, min(25.0 / nyquist, 0.4), btype="highpass", output="sos")
        out = signal.sosfilt(sos_dc, out)

    # 4. Cockpit Metallic Comb Resonance (0% dry studio -> 50% cockpit -> 100% extreme mechanized armored resonance)
    if comb_mix > 0.01:
        d1 = int(0.0085 * sample_rate)  # 117Hz chassis reflection
        d2 = int(0.0152 * sample_rate)  # 66Hz armored hull reflection
        d3 = int(0.0042 * sample_rate)  # 238Hz metallic robot plate ring

        # Dynamic feedback scaling: at 0.5 fb is ~0.72/-0.65; at 1.0 fb rises to 0.88/-0.82
        fb1 = min(0.88, 0.50 + comb_mix * 0.38)
        fb2 = -min(0.82, 0.45 + comb_mix * 0.37)
        fb3 = min(0.78, 0.30 + comb_mix * 0.48)

        a1 = np.zeros(d1 + 1, dtype=np.float32)
        a1[0] = 1.0
        a1[d1] = fb1
        c1 = signal.lfilter([1.0], a1, out)

        a2 = np.zeros(d2 + 1, dtype=np.float32)
        a2[0] = 1.0
        a2[d2] = fb2
        c2 = signal.lfilter([1.0], a2, out)

        a3 = np.zeros(d3 + 1, dtype=np.float32)
        a3[0] = 1.0
        a3[d3] = fb3
        c3 = signal.lfilter([1.0], a3, out)

        metallic_layer = 0.45 * c1 + 0.35 * c2 + 0.20 * c3
        wet_mix = min(0.95, comb_mix * 0.95)
        out = (1.0 - wet_mix) * out + wet_mix * metallic_layer

    # 5. Flanger
    if flanger_mix > 0.01:
        out = apply_flanger(out, sample_rate, rate_hz=0.6, depth_ms=1.8, base_delay_ms=2.5, mix=flanger_mix * 0.4)

    # 6. Opening / Closing Chimes
    open_tone = generate_opening_chime(opening_chime, sample_rate)
    close_tone = generate_closing_chime(closing_chime, sample_rate)

    parts = []
    if len(open_tone) > 0:
        parts.append(open_tone)
    parts.append(out)
    if len(close_tone) > 0:
        parts.append(close_tone)

    out = np.concatenate(parts)

    peak = float(np.max(np.abs(out)))
    if peak > 1e-4:
        out = (out / peak) * 0.65

    out_final = out.astype(np.float32)
    rms = float(np.sqrt(np.mean(out_final**2)))
    logger.info(
        "apply_custom_dsp: samples=%d, pitch=%.1f, sub_bass=%.2f, comb=%.2f, bandpass=%s, drive=%.2f -> out=%d, peak=%.3f, rms=%.3f",
        len(samples), pitch, sub_bass, comb_mix, radio_bandpass, radio_drive, len(out_final), float(np.max(np.abs(out_final))), rms
    )
    return out_final


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
        actual_pitch = pitch if abs(pitch) > 0.01 else -4.0
        return apply_mech_effect(
            samples, sample_rate, pitch_semitones=actual_pitch
        )

    elif effect_norm in ("radio", "tactical_radio", "tactical", "comms"):
        return apply_radio_effect(samples, sample_rate, pitch_semitones=pitch)

    elif effect_norm in ("eva", "cyberpunk_eva", "ship_ai", "spaceship"):
        actual_pitch = pitch if abs(pitch) > 0.01 else 1.0
        return apply_eva_effect(
            samples, sample_rate, pitch_semitones=actual_pitch
        )

    elif effect_norm in ("flight_deck", "atc", "aviation", "cockpit"):
        return apply_flight_deck_effect(
            samples, sample_rate, pitch_semitones=pitch
        )

    else:
        if abs(pitch) > 0.01:
            shifted = pitch_shift(samples, sample_rate, pitch)
            peak = np.max(np.abs(shifted))
            if peak > 1e-4:
                shifted = (shifted / peak) * 0.65
            return shifted

        peak = np.max(np.abs(samples))
        if peak > 1e-4:
            return (samples / peak * 0.65).astype(np.float32)
        return samples
