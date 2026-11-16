//! Whether people who already use Voquill should see the setup screen again
//! after an update.
//!
//! `SETUP_REVISION` is bumped deliberately, in a release whose setup is
//! worth seeing again, never automatically per version. Every user whose
//! completed revision is behind sees setup once as a review, whichever
//! version they updated from and whatever engine they run: it shows the
//! choice recommended for their hardware (selected, for Local users) and
//! says whether they already run it. The revision is recorded only when the
//! user finishes the review or keeps their setup, so a review that was
//! closed unanswered opens again on the next launch.

use crate::config::{Config, TranscriptionMode};
use crate::setup_recommendation::{SetupOption, SetupRecommendation};
use serde::Serialize;

/// Bump when setup has something new for existing users to review.
/// Revision 3: revisions 1 and 2 skipped everyone already on an offered or
/// the recommended choice, so most people updating never saw the review.
pub const SETUP_REVISION: u32 = 3;

#[derive(Debug, Serialize)]
pub struct SetupReview {
    /// Show the setup screen as a review after an update.
    pub due: bool,
    /// The choice setup recommends for this computer, which the review
    /// selects. None for cloud users (their engine was chosen deliberately)
    /// or when the hardware could not be checked.
    pub recommended: Option<SetupOption>,
    /// The user already runs the recommended choice.
    pub already_recommended: bool,
}

impl SetupReview {
    pub fn not_due() -> Self {
        SetupReview {
            due: false,
            recommended: None,
            already_recommended: false,
        }
    }
}

/// Whether the completed setup revision is behind the current one. Cheap, so
/// callers check it before the slower hardware detection.
pub fn revision_is_behind(config: &Config) -> bool {
    config.completed_setup_revision < SETUP_REVISION
}

/// The review for a user whose revision is behind: always due. A failed
/// hardware check (`recommendation` is None) still shows the review; the
/// setup screen reports the error itself.
pub fn review(config: &Config, recommendation: Option<SetupRecommendation>) -> SetupReview {
    let recommended = match config.transcription_mode {
        TranscriptionMode::Local => {
            recommendation.and_then(|recommendation| recommendation.options.into_iter().next())
        }
        TranscriptionMode::Api => None,
    };
    let already_recommended = recommended.as_ref().is_some_and(|first| {
        first.engine.name() == config.local_engine && first.model_size == config.local_model_size
    });
    SetupReview {
        due: true,
        recommended,
        already_recommended,
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
        let review = review(&config("Whisper.cpp", "small.en"), Some(recommendation()));
        assert!(review.due);
        assert!(!review.already_recommended);
        assert_eq!(
            review.recommended.unwrap().engine,
            crate::engines::ModelEngine::ParakeetCpu
        );
    }

    #[test]
    fn review_is_due_when_already_running_the_first_choice() {
        let review = review(
            &config("Parakeet", "parakeet-unified-en-0.6b"),
            Some(recommendation()),
        );
        assert!(review.due);
        assert!(review.already_recommended);
    }

    #[test]
    fn review_selects_the_graphics_card_over_a_lesser_choice() {
        let review = review(
            &config("Parakeet", "parakeet-unified-en-0.6b"),
            Some(graphics_card_recommendation()),
        );
        assert!(review.due);
        assert!(!review.already_recommended);
        assert_eq!(
            review.recommended.unwrap().engine,
            crate::engines::ModelEngine::ParakeetGpu
        );
    }

    #[test]
    fn cloud_users_are_reviewed_without_switching_engine() {
        let mut cloud = config("Whisper.cpp", "small.en");
        cloud.transcription_mode = TranscriptionMode::Api;
        let review = review(&cloud, Some(recommendation()));
        assert!(review.due);
        assert!(review.recommended.is_none());
    }

    #[test]
    fn a_failed_hardware_check_still_shows_the_review() {
        let review = review(&config("Whisper.cpp", "small.en"), None);
        assert!(review.due);
        assert!(review.recommended.is_none());
    }
}
