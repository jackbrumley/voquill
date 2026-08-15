"""
Audio enhancement: noise reduction using noisereduce library.

Uses spectral gating to reduce background noise from audio files.
The noise profile is estimated from the first 500ms of audio (assumed
to contain only background noise), then applied to the full signal.

Can be used as a pre-processing step before transcription to improve
accuracy in noisy environments.
"""

from __future__ import annotations

import logging
import os
import tempfile

from .schemas import EnhanceResponse

logger = logging.getLogger("voquill.enhancement.noise")

_NOISE_PROFILE_DURATION_MS = 500


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

    The noise profile is estimated from the first 500ms of audio.
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

    clip_to_sample = max(1, int(sample_rate * _NOISE_PROFILE_DURATION_MS / 1000))
    noise_profile = samples[: min(clip_to_sample, len(samples))]
    if len(noise_profile) == 0:
        logger.warning("Audio too short for noise profiling, using full signal")
        noise_profile = samples

    enhanced = noisereduce.reduce_noise(
        y=samples,
        sr=sample_rate,
        y_noise=noise_profile,
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