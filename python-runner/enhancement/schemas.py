from pydantic import BaseModel


class EnhanceRequest(BaseModel):
    audio_path: str
    noise_reduction_strength: float = 0.7


class EnhanceResponse(BaseModel):
    enhanced_path: str
    provider: str