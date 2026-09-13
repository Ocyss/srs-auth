#![allow(non_snake_case)]
#![allow(dead_code)] // TODO: remove this when the code is more complete
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dioxus::prelude::*;

use views::Home;
mod api;
mod components;
mod models;
#[cfg(feature = "server")]
mod server;
mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[route("/")]
    Home {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

#[cfg(feature = "server")]
async fn auth(
    dioxus::server::axum::Extension(state): server::AppStateEx,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, HttpError> {
    use dioxus::fullstack::{headers::HeaderMapExt, Cookie};

    let ck: Option<Cookie> = request.headers().typed_get();

    if let Some(Some(token)) = ck.as_ref().map(|c| c.get("token")) {
        if let Ok(session) = server::security::verify_session_token(token) {
            let account = state
                .db
                .find_account(&session.sub)
                .and_then(|account| account.ok_or(anyhow::anyhow!("account not found")));
            match account {
                Ok(account) if !account.is_disabled_at(chrono::Utc::now()) => {
                    request.extensions_mut().insert(server::AuthEx {
                        account,
                        session_claims: session,
                    });
                }
                Ok(_) => {
                    return Err(HttpError::new(
                        StatusCode::UNAUTHORIZED,
                        "session is invalid or account is disabled",
                    ));
                }
                Err(error) => {
                    tracing::warn!(%error, "failed to resolve account for session");
                    return Err(HttpError::new(
                        StatusCode::UNAUTHORIZED,
                        "failed to resolve account for session",
                    ));
                }
            }
        }
    }
    Ok(next.run(request).await)
}

fn main() {
    #[cfg(feature = "server")]
    {
        dotenvy::dotenv().ok();
        dioxus::serve(|| async move {
            use dioxus::server::axum::Extension;
            let app_state = server::AppState::new().await?;
            Ok(dioxus::server::router(App)
                .layer(axum::middleware::from_fn(auth))
                .layer(Extension(std::sync::Arc::new(app_state))))
        });
    }

    #[cfg(not(feature = "server"))]
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        // script { src: "https://testingcf.jsdelivr.net/npm/ovenplayer/dist/ovenplayer.js" }
        link {
            rel: "stylesheet",
            href: "https://unpkg.byted-static.com/xgplayer/3.0.24/dist/index.min.css",
        }
        script { src: "https://unpkg.byted-static.com/xgplayer/3.0.24/dist/index.min.js" }
        script { src: "https://unpkg.byted-static.com/xgplayer-flv/3.0.24/dist/index.min.js" }

        Router::<Route> {}
    }
}
