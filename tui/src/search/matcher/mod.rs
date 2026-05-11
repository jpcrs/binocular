pub mod exact;
pub mod fuzzy;

pub use exact::spawn_exact_matcher;
pub use fuzzy::{spawn_matcher, MatcherCommand, MatcherState};
