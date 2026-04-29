pub mod documents;
pub mod envelopes;
pub mod health;

use axum::Router;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(health::routes())
        .merge(documents::routes())
        .merge(envelopes::routes())
}
