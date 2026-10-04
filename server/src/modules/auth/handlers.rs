use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use axum_extra::extract::{
    CookieJar,
    cookie::{Cookie, SameSite},
};

use super::{
    AUTH_COOKIE, AuthUser,
    dto::{LoginRequest, LoginResponse, SignupRequest},
    service,
};
use crate::{
    error::{AppError, AppResult},
    modules::users::PublicUser,
    shared::{login_throttle::Attempt, validation::normalize_handle},
    state::AppState,
};

pub async fn signup(
    State(state): State<AppState>,
    caller: Result<AuthUser, AppError>,
    Json(req): Json<SignupRequest>,
) -> AppResult<(StatusCode, Json<PublicUser>)> {
    let user = service::signup(&state.db, req, caller.is_ok()).await?;
    Ok((StatusCode::CREATED, Json(user.into())))
}

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(req): Json<LoginRequest>,
) -> AppResult<(CookieJar, Json<LoginResponse>)> {
    let attempt = Attempt::new("admin", &normalize_handle(&req.username), &headers);
    state.login_throttle.check(&attempt)?;
    let (user, token) = match service::login(&state.db, &state.config, req).await {
        Ok(ok) => {
            state.login_throttle.record_success(&attempt);
            ok
        }
        Err(e) => {
            if matches!(e, AppError::Unauthorized(_)) {
                state.login_throttle.record_failure(&attempt);
            }
            return Err(e);
        }
    };

    let mut cookie = auth_cookie(&state, token.clone());
    cookie.set_max_age(time::Duration::hours(state.config.jwt_ttl_hours));

    Ok((
        jar.add(cookie),
        Json(LoginResponse {
            token,
            user: user.into(),
        }),
    ))
}

pub async fn logout(State(state): State<AppState>, jar: CookieJar) -> (CookieJar, StatusCode) {
    let cookie = auth_cookie(&state, String::new());
    (jar.remove(cookie), StatusCode::NO_CONTENT)
}

pub async fn me(State(state): State<AppState>, auth: AuthUser) -> AppResult<Json<PublicUser>> {
    let user = service::current_user(&state.db, auth.id).await?;
    Ok(Json(user.into()))
}

fn auth_cookie(state: &AppState, value: String) -> Cookie<'static> {
    Cookie::build((AUTH_COOKIE, value))
        .http_only(true)
        .secure(state.config.cookie_secure)
        .same_site(SameSite::Lax)
        .path("/")
        .build()
}
