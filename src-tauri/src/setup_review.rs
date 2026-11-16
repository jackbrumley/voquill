//! Whether people who already use Voquill should see the setup screen again
//! after an update.
//!
//! `SETUP_REVISION` is bumped deliberately, in a release whose setup offers
//! something worth seeing again (such as a better engine recommendation),
//! never automatically per version. A user whose completed revision is
//! behind is shown setup once, but only if it has something to offer: the
//! first (recommended) choice differs from the engine they run. Otherwise the
//! revision is recorded silently.

use crate::config::{Config, TranscriptionMode};
use crate::setup_recommendation::{SetupOption, SetupRecommendation};
use serde::Serialize;

/// Bump when setup has something new for existing users to review.
/// Revision 2: revision 1 skipped users on a lesser offered choice (the
/// processor model on a computer with a suitable graphics card).
pub const SETUP_REVISION: u32 = 2;

#[derive(Debug, Serialize)]
pub struct SetupReview {
    /// Show the setup screen as a review of new options.
    pub due: bool,
    /// The choice setup would recommend, when the review is due.
    pub recommended: Option<SetupOption>,
}

/// Whether the completed setup revision is behind the current one. Cheap, so
/// callers check it before the slower hardware detection.
pub fn revision_is_behind(config: &Config) -> bool {
    config.completed_setup_revision < SETUP_REVISION
}

/// Decides the review for a user whose revision is behind. Cloud users chose
/// their engine deliberately, and a user already running the recommended
/// (first) choice has nothing new to see. A user on a lesser offered choice
/// is reviewed; "Keep my current setup" keeps it if they chose it on purpose.
pub fn review(config: &Config, recommendation: SetupRecommendation) -> SetupReview {
    let recommended = recommendation.options.into_iter().next().filter(|first| {
        let runs_first_choice = first.engine.name() == config.local_engine
            && first.model_size == config.local_model_size;
        config.transcription_mode == TranscriptionMode::Local && !runs_first_choice
    });
    SetupReview {
        due: recommended.is_some(),
        recommended,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn option(engine: crate::engines::ModelEngine, model_size: &str) -> SetupOption {
        SetupOption {
            engine,
            model_size: model_size.to_string(),
            title: String::new(),
            description: String::new(),
            download_bytes: 0,
            speed: 2,
            accuracy: 4,
        }
    }

    fn recommendation() -> SetupRecommendation {
        SetupRecommendation {
            graphics_card: None,
            options: vec![option(
                crate::engines::ModelEngine::ParakeetCpu,
                "parakeet-unified-en-0.6b",
            )],
            explanation: String::new(),
        }
    }

    /// A computer with a suitable graphics card: the GPU choice first, the
    /// processor choice second.
    fn graphics_card_recommendation() -> SetupRecommendation {
        SetupRecommendation {
            graphics_card: Some("NVIDIA RTX A2000 12GB".to_string()),
            options: vec![
                option(crate::engines::ModelEngine::ParakeetGpu, "tdt-0.6b-v2-q8_0"),
                option(
                    crate::engines::ModelEngine::ParakeetCpu,
                    "parakeet-unified-en-0.6b",
                ),
            ],
            explanation: String::new(),
        }
    }

    fn config(engine: &str, model: &str) -> Config {
        Config {
            local_engine: engine.to_string(),
            local_model_size: model.to_string(),
            transcription_mode: TranscriptionMode::Local,
            ..Config::default()
        }
    }

    #[test]
    fn new_and_upgraded_configs_are_behind() {
        assert!(revision_is_behind(&Config::default()));
        let config = Config {
            completed_setup_revision: SETUP_REVISION,
            ..Config::default()
        };
        assert!(!revision_is_behind(&config));
    }

    #[test]
    fn review_is_due_when_the_recommendation_differs() {
        let review = review(&config("Whisper.cpp", "small.en"), recommendation());
        assert!(review.due);
        assert_eq!(
            review.recommended.unwrap().engine,
            crate::engines::ModelEngine::ParakeetCpu
        );
    }

    #[test]
    fn no_review_when_already_running_the_first_choice() {
        let review = review(
            &config("Parakeet", "parakeet-unified-en-0.6b"),
            recommendation(),
        );
        assert!(!review.due);
    }

    #[test]
    fn review_is_due_when_running_a_lesser_offered_choice() {
        let review = review(
            &config("Parakeet", "parakeet-unified-en-0.6b"),
            graphics_card_recommendation(),
        );
        assert!(review.due);
        assert_eq!(
            review.recommended.unwrap().engine,
            crate::engines::ModelEngine::ParakeetGpu
        );
    }

    #[test]
    fn cloud_users_are_not_reviewed() {
        let mut cloud = config("Whisper.cpp", "small.en");
        cloud.transcription_mode = TranscriptionMode::Api;
        assert!(!review(&cloud, recommendation()).due);
    }
}
