//! Whether people who already use Voquill should see the setup screen again
//! after an update.
//!
//! `SETUP_REVISION` is bumped deliberately, in a release whose setup offers
//! something worth seeing again (such as a better engine recommendation),
//! never automatically per version. A user whose completed revision is
//! behind is shown setup once, but only if it has something to offer: the
//! recommendation differs from the engine they run. Otherwise the revision
//! is recorded silently.

use crate::config::{Config, TranscriptionMode};
use crate::setup_recommendation::{SetupOption, SetupRecommendation};
use serde::Serialize;

/// Bump when setup has something new for existing users to review.
pub const SETUP_REVISION: u32 = 1;

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
/// their engine deliberately, and a user already running any offered choice
/// has nothing new to see.
pub fn review(config: &Config, recommendation: SetupRecommendation) -> SetupReview {
    let runs_an_offered_choice = recommendation.options.iter().any(|option| {
        option.engine.name() == config.local_engine && option.model_size == config.local_model_size
    });
    let has_something_new =
        config.transcription_mode == TranscriptionMode::Local && !runs_an_offered_choice;
    SetupReview {
        due: has_something_new,
        recommended: if has_something_new {
            recommendation.options.into_iter().next()
        } else {
            None
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recommendation() -> SetupRecommendation {
        SetupRecommendation {
            graphics_card: None,
            options: vec![SetupOption {
                engine: crate::engines::ModelEngine::ParakeetCpu,
                model_size: "parakeet-unified-en-0.6b".to_string(),
                title: "Works on any computer".to_string(),
                description: String::new(),
                download_bytes: 0,
                speed: 2,
                accuracy: 4,
            }],
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
    fn no_review_when_already_running_an_offered_choice() {
        let review = review(
            &config("Parakeet", "parakeet-unified-en-0.6b"),
            recommendation(),
        );
        assert!(!review.due);
    }

    #[test]
    fn cloud_users_are_not_reviewed() {
        let mut cloud = config("Whisper.cpp", "small.en");
        cloud.transcription_mode = TranscriptionMode::Api;
        assert!(!review(&cloud, recommendation()).due);
    }
}
