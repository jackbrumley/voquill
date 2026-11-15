//! The built-in voice personas: a base voice plus the effect, pitch and
//! speed that give it its character. Moved here from the Python runner so
//! Rust owns every voice decision and Python only renders.

pub struct Persona {
    pub id: &'static str,
    pub name: &'static str,
    pub persona: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub base_model: &'static str,
    pub default_effect: &'static str,
    pub default_pitch: f32,
    pub default_speed: f32,
}

pub const PERSONAS: &[Persona] = &[
    Persona {
        id: "tactical-comms",
        name: "Tactical Comms",
        persona: "SAS Tactical Operator",
        category: "Military & Tactical",
        description: "Northern English military comms with authentic VHF bandpass filtering, tactical overdrive, and tail squelch.",
        base_model: "piper-en_GB-northern_english_male-medium",
        default_effect: "radio",
        default_pitch: 0.0,
        default_speed: 1.05,
    },
    Persona {
        id: "titan-mech",
        name: "Titan Mech",
        persona: "Armored Cockpit AI",
        category: "Sci-Fi & Gaming",
        description: "Deep, authoritative pilot system with metallic armored chassis resonance and sub-bass weight.",
        base_model: "piper-en_US-ryan-low",
        default_effect: "mech",
        default_pitch: -4.0,
        default_speed: 0.95,
    },
    Persona {
        id: "nanosuit",
        name: "Nanosuit AI",
        persona: "Tactical Combat Exosuit",
        category: "Sci-Fi & Gaming",
        description: "Cybernetic combat armor system with synthetic pitch modulation and power-shield resonance.",
        base_model: "piper-en_US-ryan-low",
        default_effect: "mech",
        default_pitch: -2.0,
        default_speed: 1.0,
    },
    Persona {
        id: "glados",
        name: "GLaDOS AI",
        persona: "Iconic Robot AI",
        category: "Sci-Fi & Gaming",
        description: "Iconic robotic AI with distinctive robotic inflections and dry demeanor.",
        base_model: "piper-en_US-glados",
        default_effect: "clean",
        default_pitch: 0.0,
        default_speed: 1.0,
    },
    Persona {
        id: "cyberpunk-eva",
        name: "Cyberpunk EVA",
        persona: "Holographic Ship AI",
        category: "Sci-Fi & Gaming",
        description: "Futuristic spacecraft computer with holographic bridge reflections and crystal air clarity.",
        base_model: "piper-en_US-amy-low",
        default_effect: "eva",
        default_pitch: 1.0,
        default_speed: 1.0,
    },
    Persona {
        id: "flight-deck",
        name: "Flight Deck ATC",
        persona: "British Flight Controller",
        category: "Aviation & Simulation",
        description: "Crisp British aviation air traffic control / cockpit automated warning system.",
        base_model: "piper-en_GB-southern_english_female-low",
        default_effect: "flight_deck",
        default_pitch: 0.0,
        default_speed: 1.0,
    },
    Persona {
        id: "nova-studio",
        name: "Nova Studio (Female)",
        persona: "Clean Studio Female",
        category: "Studio & Natural",
        description: "Natural, crystal-clear studio narration voice with zero acoustic coloration.",
        base_model: "piper-en_US-lessac-low",
        default_effect: "clean",
        default_pitch: 0.0,
        default_speed: 1.0,
    },
    Persona {
        id: "apex-studio",
        name: "Apex Studio (Male)",
        persona: "Authoritative Studio Male",
        category: "Studio & Natural",
        description: "Clean, warm, and natural male studio voice for desktop automation and productivity.",
        base_model: "piper-en_US-ryan-low",
        default_effect: "clean",
        default_pitch: 0.0,
        default_speed: 1.0,
    },
];

/// Voice IDs saved by older versions, which named base voices directly.
const LEGACY_ALIASES: &[(&str, &str)] = &[
    ("piper-en_US-amy-low", "cyberpunk-eva"),
    ("piper-en_US-glados", "glados"),
    ("piper-en_US-ryan-low", "titan-mech"),
    ("piper-en_GB-southern_english_female-low", "flight-deck"),
    ("piper-en_US-arctic-medium", "tactical-comms"),
    ("piper-en_US-lessac-low", "nova-studio"),
];

/// The persona for a voice ID, following legacy aliases.
pub fn find(voice_id: &str) -> Option<&'static Persona> {
    let canonical = LEGACY_ALIASES
        .iter()
        .find(|(legacy, _)| *legacy == voice_id)
        .map(|(_, persona)| *persona)
        .unwrap_or(voice_id);
    PERSONAS.iter().find(|persona| persona.id == canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::python_runner::tts_models::TTS_BASE_MODELS;

    #[test]
    fn every_persona_uses_a_known_base_voice() {
        for persona in PERSONAS {
            assert!(
                TTS_BASE_MODELS
                    .iter()
                    .any(|model| model.key == persona.base_model),
                "{} uses unknown voice {}",
                persona.id,
                persona.base_model
            );
        }
    }

    #[test]
    fn legacy_ids_resolve_to_personas() {
        assert_eq!(find("piper-en_US-glados").unwrap().id, "glados");
        assert_eq!(
            find("piper-en_US-arctic-medium").unwrap().id,
            "tactical-comms"
        );
        assert!(find("not-a-voice").is_none());
    }
}
