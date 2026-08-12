"""
Tier 2 diarization: sherpa-onnx speaker embedding + clustering.

sherpa-onnx is a lightweight C++ library with Python bindings that does NOT
require PyTorch. Models are cached in the models/ subdirectory.

Tier 3 (pyannote-audio + torch) would be added as provider_pyannote.py
with the same run() signature — no Rust changes needed.
"""

from __future__ import annotations

import logging
import os
from pathlib import Path

from .schemas import DiarizeResponse, Segment

logger = logging.getLogger("voquill.diarization.sherpa")


def is_available() -> bool:
    try:
        import sherpa_onnx  # noqa: F401

        return True
    except ImportError:
        return False


def _ensure_model_dir(base_dir: str) -> str:
    models_dir = os.path.join(base_dir, "models")
    os.makedirs(models_dir, exist_ok=True)
    return models_dir


def run(audio_path: str, runner_base_dir: str) -> DiarizeResponse:
    """
    Run speaker diarization on audio_path using sherpa-onnx.

    runner_base_dir is the python-runner root directory
    (where .version lives, and where models/ is created).
    """
    import sherpa_onnx

    model_dir = _ensure_model_dir(runner_base_dir)

    # --- Build the diarization config ---
    # These models are auto-downloaded by sherpa-onnx on first use.
    # They are small ONNX models (~20-50MB total).
    config = sherpa_onnx.OfflineSpeakerDiarizationConfig(
        segmentation=sherpa_onnx.OfflineSpeakerSegmentationConfig(
            model_type="pyannote",
            model_config=sherpa_onnx.SpeakerSegmentationModelConfig(
                model_type="pyannote",
            ),
        ),
        embedding=sherpa_onnx.SpeakerEmbeddingExtractorConfig(
            model_type="wespeaker",
            model_config=sherpa_onnx.SpeakerEmbeddingModelConfig(
                model_type="wespeaker",
                model="3dspeaker_speech_eres2net_base_sv_zh-cn_3dspeaker_16k",
            ),
        ),
        clustering=sherpa_onnx.SpeakerEmbeddingClusteringConfig(
            threshold=0.5,
            min_num_speakers=1,
            max_num_speakers=5,
        ),
        model_dir=model_dir,
    )

    diarizer = sherpa_onnx.OfflineSpeakerDiarization(config)

    # --- Process audio ---
    import soundfile as sf

    samples, sample_rate = sf.read(audio_path, dtype="float32")
    if len(samples.shape) > 1:
        samples = samples.mean(axis=1)

    result = diarizer.process(samples, sample_rate)

    # --- Build response ---
    segments = []
    speaker_map: dict[int, str] = {}
    next_label = 1

    for seg in result:
        speaker_id = seg.speaker
        if speaker_id not in speaker_map:
            speaker_map[speaker_id] = f"Person {next_label}"
            next_label += 1

        label = speaker_map[speaker_id]
        segments.append(
            Segment(
                speaker=label,
                text=seg.text.strip() if seg.text else "",
                start_sec=seg.start,
                end_sec=seg.end,
            )
        )

    return DiarizeResponse(segments=segments, provider="sherpa-onnx")