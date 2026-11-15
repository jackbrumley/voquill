from __future__ import annotations

from typing import Literal, Optional, Union

from pydantic import BaseModel, Field


class EffectDsp(BaseModel):
    """A named persona effect (radio, mech, eva, ...) with a pitch shift."""

    kind: Literal["effect"]
    effect: str
    pitch: float = 0.0


class CustomDsp(BaseModel):
    """Voice Studio's individual tuning knobs."""

    kind: Literal["custom"]
    pitch: float = 0.0
    sub_bass: float = 0.0
    comb_mix: float = 0.0
    flanger_mix: float = 0.0
    radio_bandpass: bool = False
    radio_drive: float = 1.0
    rf_noise: float = 0.0
    opening_chime: str = "none"
    closing_chime: str = "none"


class TtsRenderRequest(BaseModel):
    """A fully resolved render: Rust has already chosen the voice, located
    (and downloaded) its files, and set every parameter."""

    text: str
    model_path: str
    tokens_path: str
    data_dir: str
    speaker_id: int = 0
    speed: float = 1.0
    noise_scale: float = 0.667
    dsp: Union[EffectDsp, CustomDsp] = Field(discriminator="kind")
    output_path: Optional[str] = None


class TtsSynthesizeResponse(BaseModel):
    output_path: str
    duration_secs: float
    sample_rate: int
    provider: str
