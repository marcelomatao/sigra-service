pub mod documents;
pub mod envelopes;
pub mod health;
pub mod signing;

use axum::Router;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(health::routes())
        .merge(documents::routes())
        .merge(envelopes::routes())
        .merge(signing::routes())
}
