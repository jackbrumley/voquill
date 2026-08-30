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
import tarfile
import urllib.request

from .schemas import DiarizeResponse, Segment

logger = logging.getLogger("voquill.diarization.sherpa")

_SEGMENTATION_MODEL_URL = (
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/"
    "speaker-segmentation-models/"
    "sherpa-onnx-pyannote-segmentation-3-0.tar.bz2"
)
_SEGMENTATION_MODEL_DIR = "sherpa-onnx-pyannote-segmentation-3-0"
_SEGMENTATION_MODEL_FILE = "model.onnx"

_EMBEDDING_MODEL_URL = (
    "https://github.com/k2-fsa/sherpa-onnx/releases/download/"
    "speaker-recongition-models/"
    "3dspeaker_speech_eres2net_base_sv_zh-cn_3dspeaker_16k.onnx"
)
_EMBEDDING_MODEL_FILE = "3dspeaker_speech_eres2net_base_sv_zh-cn_3dspeaker_16k.onnx"


def _ensure_segmentation_model(runner_base_dir: str) -> str:
    model_path = os.path.join(
        runner_base_dir,
        "models",
        _SEGMENTATION_MODEL_DIR,
        _SEGMENTATION_MODEL_FILE,
    )
    if os.path.exists(model_path):
        return model_path

    models_dir = os.path.join(runner_base_dir, "models")
    os.makedirs(models_dir, exist_ok=True)

    archive_name = f"{_SEGMENTATION_MODEL_DIR}.tar.bz2"
    archive_path = os.path.join(models_dir, archive_name)

    logger.info("Downloading speaker segmentation model fallback...")
    try:
        urllib.request.urlretrieve(_SEGMENTATION_MODEL_URL, archive_path)
        logger.info("Extracting speaker segmentation model...")
        with tarfile.open(archive_path, "r:bz2") as tar:
            tar.extractall(path=models_dir)
    finally:
        if os.path.exists(archive_path):
            try:
                os.remove(archive_path)
            except OSError:
                pass

    if not os.path.exists(model_path):
        raise FileNotFoundError(
            f"Segmentation model not found after extraction at {model_path}"
        )

    logger.info("Speaker segmentation model ready at %s", model_path)
    return model_path


def _ensure_embedding_model(runner_base_dir: str) -> str:
    model_path = os.path.join(
        runner_base_dir,
        "models",
        _EMBEDDING_MODEL_FILE,
    )
    if os.path.exists(model_path):
        return model_path

    models_dir = os.path.join(runner_base_dir, "models")
    os.makedirs(models_dir, exist_ok=True)

    logger.info("Downloading speaker embedding model fallback...")
    try:
        urllib.request.urlretrieve(_EMBEDDING_MODEL_URL, model_path)
    except Exception as e:
        if os.path.exists(model_path):
            try:
                os.remove(model_path)
            except OSError:
                pass
        raise FileNotFoundError(
            f"Embedding model not found after download attempt at {model_path}: {e}"
        ) from e

    logger.info("Speaker embedding model ready at %s", model_path)
    return model_path


def is_available() -> bool:
    try:
        import sherpa_onnx  # noqa: F401

        return True
    except ImportError:
        return False


def run(audio_path: str, runner_base_dir: str, cluster_threshold: float = 0.5) -> DiarizeResponse:
    """
    Run speaker diarization on audio_path using sherpa-onnx.

    runner_base_dir is the python-runner root directory
    (where .version lives, and where models/ is created).

    cluster_threshold controls how aggressively speakers are merged:
    a larger threshold produces fewer speakers, a smaller one more.
    """
    import sherpa_onnx
    import numpy as np

    segmentation_model = _ensure_segmentation_model(runner_base_dir)
    embedding_model = _ensure_embedding_model(runner_base_dir)

    # --- Build the diarization config ---
    config = sherpa_onnx.OfflineSpeakerDiarizationConfig(
        segmentation=sherpa_onnx.OfflineSpeakerSegmentationModelConfig(
            pyannote=sherpa_onnx.OfflineSpeakerSegmentationPyannoteModelConfig(
                model=segmentation_model,
            ),
        ),
        embedding=sherpa_onnx.SpeakerEmbeddingExtractorConfig(
            model=embedding_model,
        ),
        clustering=sherpa_onnx.FastClusteringConfig(
            threshold=cluster_threshold,
        ),
        min_duration_on=0.3,
        min_duration_off=0.5,
    )

    diarizer = sherpa_onnx.OfflineSpeakerDiarization(config)

    # --- Process audio ---
    import soundfile as sf

    samples, sample_rate = sf.read(audio_path, dtype="float32")
    if len(samples.shape) > 1:
        samples = samples.mean(axis=1)

    # Resample to 16kHz (sherpa-onnx models expect this)
    if sample_rate != 16000:
        duration = len(samples) / sample_rate
        target_len = int(duration * 16000)
        indices = np.linspace(0, len(samples) - 1, target_len)
        samples = np.interp(indices, np.arange(len(samples)), samples).astype(np.float32)

    result = diarizer.process(samples)

    # --- Build response ---
    segments = []
    speaker_map: dict[int, str] = {}
    next_label = 1

    for seg in result.sort_by_start_time():
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