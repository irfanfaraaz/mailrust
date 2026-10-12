use hickory_resolver::{
    TokioAsyncResolver,
    proto::rr::{RData, RecordType},
};

/// Returns the CNAME target of `host` without the trailing dot, if any.
pub async fn cname_target(host: &str) -> Option<String> {
    let resolver = TokioAsyncResolver::tokio_from_system_conf().ok()?;
    let lookup = resolver.lookup(host, RecordType::CNAME).await.ok()?;
    lookup.record_iter().find_map(|r| match r.data() {
        Some(RData::CNAME(c)) => Some(c.0.to_utf8().trim_end_matches('.').to_ascii_lowercase()),
        _ => None,
    })
}

pub async fn cname_points_to(host: &str, target: &str) -> bool {
    cname_target(host)
        .await
        .is_some_and(|t| t.eq_ignore_ascii_case(target.trim_end_matches('.')))
}
