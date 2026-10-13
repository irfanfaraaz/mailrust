mod common;

use axum::http::StatusCode;
use mailrust::users::{self, MemberRole};
use serde_json::json;
use uuid::Uuid;

struct Team {
    t: common::TestApp,
    owner: String,
    pool: sqlx::PgPool,
}

async fn team(pool: sqlx::PgPool) -> Team {
    let mut t = common::app(pool.clone()).await;
    common::with_system_sender(&mut t, "login@mail.shop.com").await;
    let owner = common::session_for(&pool, t.account.id, "owner@shop.com", MemberRole::Owner).await;
    Team { t, owner, pool }
}

impl Team {
    async fn call(
        &self,
        s: &str,
        method: &str,
        uri: &str,
        body: Option<serde_json::Value>,
    ) -> (StatusCode, serde_json::Value) {
        common::call(
            &self.t.router,
            common::sreq(method, uri, s, Some(self.t.account.id), body),
        )
        .await
    }
    async fn invite_token(&self, email: &str) -> String {
        let text: String = sqlx::query_scalar("SELECT text_body FROM emails WHERE $1 = ANY(to_addresses) ORDER BY created_at DESC LIMIT 1")
            .bind(email).fetch_one(&self.pool).await.unwrap();
        let i = text.find("token=").unwrap() + 6;
        text[i..].split_whitespace().next().unwrap().to_string()
    }
}

#[sqlx::test]
async fn an_owner_invites_and_the_new_person_joins_as_member(pool: sqlx::PgPool) {
    let tm = team(pool).await;
    let (s, b) = tm
        .call(
            &tm.owner,
            "POST",
            "/team/invites",
            Some(json!({"email": "Bo@Shop.com", "role": "member"})),
        )
        .await;
    assert_eq!(s, StatusCode::CREATED, "{b}");
    assert_eq!(b["email"], "bo@shop.com");
    let token = tm.invite_token("bo@shop.com").await;
    assert!(token.starts_with("mr_inv_"));
    let (s, b) = common::call(
        &tm.t.router,
        common::req(
            "POST",
            "/invites/accept",
            None,
            Some(json!({"token": token})),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    let bo = b["session_token"].as_str().unwrap().to_string();
    let (s, b) = tm.call(&tm.owner, "GET", "/team", None).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(b["members"].as_array().unwrap().len(), 2);
    assert!(b["invites"].as_array().unwrap().is_empty());
    let (s, _) = tm.call(&bo, "GET", "/team", None).await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = common::call(
        &tm.t.router,
        common::req(
            "POST",
            "/invites/accept",
            None,
            Some(json!({"token": token})),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED, "an invite works once");
}

#[sqlx::test]
async fn a_member_cannot_manage_the_team_and_api_keys_cannot_either(pool: sqlx::PgPool) {
    let tm = team(pool.clone()).await;
    let m = common::session_for(&pool, tm.t.account.id, "m@shop.com", MemberRole::Member).await;
    let (s, _) = tm
        .call(
            &m,
            "POST",
            "/team/invites",
            Some(json!({"email": "x@shop.com", "role": "member"})),
        )
        .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
    let (s, _) = common::call(
        &tm.t.router,
        common::req("GET", "/team", Some(&tm.t.key), None),
    )
    .await;
    assert_eq!(s, StatusCode::FORBIDDEN);
}

#[sqlx::test]
async fn the_last_owner_cannot_be_removed_or_demoted_but_others_can_leave(pool: sqlx::PgPool) {
    let tm = team(pool.clone()).await;
    let me: Uuid = users::find_by_email(&pool, "owner@shop.com")
        .await
        .unwrap()
        .unwrap()
        .id;
    let (s, b) = tm
        .call(
            &tm.owner,
            "PATCH",
            &format!("/team/members/{me}"),
            Some(json!({"role": "member"})),
        )
        .await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(b["error"]["code"], "last_owner");
    let (s, _) = tm
        .call(&tm.owner, "DELETE", &format!("/team/members/{me}"), None)
        .await;
    assert_eq!(s, StatusCode::CONFLICT);

    let m = common::session_for(&pool, tm.t.account.id, "m@shop.com", MemberRole::Member).await;
    let mid = users::find_by_email(&pool, "m@shop.com")
        .await
        .unwrap()
        .unwrap()
        .id;
    let (s, _) = tm
        .call(&m, "DELETE", &format!("/team/members/{mid}"), None)
        .await;
    assert_eq!(s, StatusCode::NO_CONTENT, "a member may leave");
    let (s, _) = tm.call(&m, "GET", "/team", None).await;
    assert_eq!(s, StatusCode::NOT_FOUND, "and is gone");
}

#[sqlx::test]
async fn with_two_owners_one_can_step_down(pool: sqlx::PgPool) {
    let tm = team(pool.clone()).await;
    common::session_for(&pool, tm.t.account.id, "o2@shop.com", MemberRole::Owner).await;
    let me = users::find_by_email(&pool, "owner@shop.com")
        .await
        .unwrap()
        .unwrap()
        .id;
    let (s, b) = tm
        .call(
            &tm.owner,
            "PATCH",
            &format!("/team/members/{me}"),
            Some(json!({"role": "member"})),
        )
        .await;
    assert_eq!(s, StatusCode::OK, "{b}");
    assert_eq!(b["role"], "member");
}

#[sqlx::test]
async fn re_inviting_revokes_the_old_link_and_expired_links_fail(pool: sqlx::PgPool) {
    let tm = team(pool.clone()).await;
    tm.call(
        &tm.owner,
        "POST",
        "/team/invites",
        Some(json!({"email": "bo@shop.com", "role": "member"})),
    )
    .await;
    let old = tm.invite_token("bo@shop.com").await;
    tm.call(
        &tm.owner,
        "POST",
        "/team/invites",
        Some(json!({"email": "bo@shop.com", "role": "owner"})),
    )
    .await;
    let new = tm.invite_token("bo@shop.com").await;
    let (s, _) = common::call(
        &tm.t.router,
        common::req("POST", "/invites/accept", None, Some(json!({"token": old}))),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    sqlx::query(
        "UPDATE invites SET expires_at = now() - interval '1 second' WHERE revoked_at IS NULL",
    )
    .execute(&pool)
    .await
    .unwrap();
    let (s, _) = common::call(
        &tm.t.router,
        common::req("POST", "/invites/accept", None, Some(json!({"token": new}))),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn inviting_a_member_conflicts_and_accepting_never_lowers_a_role(pool: sqlx::PgPool) {
    let tm = team(pool.clone()).await;
    let (s, b) = tm
        .call(
            &tm.owner,
            "POST",
            "/team/invites",
            Some(json!({"email": "owner@shop.com", "role": "member"})),
        )
        .await;
    assert_eq!(s, StatusCode::CONFLICT);
    assert_eq!(b["error"]["code"], "already_member");

    // An owner of another account accepts a member invite here: becomes member here, stays owner there.
    let other = mailrust::accounts::create(&pool, "Other").await.unwrap();
    users::invite_direct(&pool, "bo@shop.com", other.id, MemberRole::Owner)
        .await
        .unwrap();
    tm.call(
        &tm.owner,
        "POST",
        "/team/invites",
        Some(json!({"email": "bo@shop.com", "role": "member"})),
    )
    .await;
    let tok = tm.invite_token("bo@shop.com").await;
    let (s, _) = common::call(
        &tm.t.router,
        common::req("POST", "/invites/accept", None, Some(json!({"token": tok}))),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let bo = users::find_by_email(&pool, "bo@shop.com")
        .await
        .unwrap()
        .unwrap()
        .id;
    assert_eq!(
        users::membership(&pool, bo, other.id)
            .await
            .unwrap()
            .unwrap()
            .role,
        MemberRole::Owner
    );
    assert_eq!(
        users::membership(&pool, bo, tm.t.account.id)
            .await
            .unwrap()
            .unwrap()
            .role,
        MemberRole::Member
    );
}

#[sqlx::test]
async fn an_owner_can_cancel_an_invite(pool: sqlx::PgPool) {
    let tm = team(pool).await;
    let (_, b) = tm
        .call(
            &tm.owner,
            "POST",
            "/team/invites",
            Some(json!({"email": "bo@shop.com", "role": "member"})),
        )
        .await;
    let id = b["id"].as_str().unwrap();
    let (s, _) = tm
        .call(&tm.owner, "DELETE", &format!("/team/invites/{id}"), None)
        .await;
    assert_eq!(s, StatusCode::NO_CONTENT);
    let tok = tm.invite_token("bo@shop.com").await;
    let (s, _) = common::call(
        &tm.t.router,
        common::req("POST", "/invites/accept", None, Some(json!({"token": tok}))),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}

#[sqlx::test]
async fn inviting_without_system_email_fails_and_creates_nothing(pool: sqlx::PgPool) {
    let t = common::app(pool.clone()).await;
    let owner = common::session_for(&pool, t.account.id, "owner@shop.com", MemberRole::Owner).await;
    let (s, b) = common::call(
        &t.router,
        common::sreq(
            "POST",
            "/team/invites",
            &owner,
            Some(t.account.id),
            Some(json!({"email": "bo@shop.com", "role": "member"})),
        ),
    )
    .await;
    assert_eq!(s, StatusCode::CONFLICT, "{b}");
    assert_eq!(b["error"]["code"], "system_email_not_set_up");
    let invites: i64 = sqlx::query_scalar("SELECT count(*) FROM invites")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(invites, 0);
}
