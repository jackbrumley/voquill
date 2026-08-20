"""
Audio enhancement: noise reduction using noisereduce library.

Uses spectral gating to reduce background noise from audio files.
Estimates the noise floor statistics across the signal without assuming
any specific time slice is silent, avoiding voice clipping when speech
starts immediately.

Can be used as a pre-processing step before transcription to improve
accuracy in noisy environments.
"""

from __future__ import annotations

import logging
import os

from .schemas import EnhanceResponse

logger = logging.getLogger("voquill.enhancement.noise")


def is_available() -> bool:
    try:
        import noisereduce  # noqa: F401
        import soundfile  # noqa: F401
        import numpy  # noqa: F401

        return True
    except ImportError:
        return False


def run(
    audio_path: str,
    runner_base_dir: str,
    noise_reduction_strength: float = 0.7,
) -> EnhanceResponse:
    """
    Reduce background noise in audio_path using spectral gating.

    Estimates stationary noise statistics across the audio signal.
    noise_reduction_strength (0.0-1.0) controls how aggressively noise
    is removed: 0.0 = no reduction, 1.0 = maximum reduction.
    Default 0.7 works well for most environments.

    Returns the path to the enhanced WAV file.
    """
    import noisereduce
    import numpy as np
    import soundfile as sf

    logger.info(
        "Enhancing audio: %s (strength=%.2f)", audio_path, noise_reduction_strength
    )

    samples, sample_rate = sf.read(audio_path, dtype="float32")
    if len(samples.shape) > 1:
        samples = samples.mean(axis=1)

    logger.info(
        "Loaded audio: %d samples at %dHz (%.1fs)",
        len(samples),
        sample_rate,
        len(samples) / sample_rate,
    )

    enhanced = noisereduce.reduce_noise(
        y=samples,
        sr=sample_rate,
        prop_decrease=noise_reduction_strength,
        stationary=True,
    )

    out_dir = os.path.join(runner_base_dir, "enhanced")
    os.makedirs(out_dir, exist_ok=True)

    base = os.path.splitext(os.path.basename(audio_path))[0]
    out_path = os.path.join(out_dir, f"{base}_enhanced.wav")

    sf.write(out_path, enhanced, sample_rate)
    logger.info("Enhanced audio saved to: %s", out_path)

    return EnhanceResponse(
        enhanced_path=out_path,
        provider="noisereduce",
    )