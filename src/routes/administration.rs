use super::{
    CookieMap, JustStringID, fetch_base_data, for_client, get_cookie_map_for_headers,
    get_cookie_map_for_req, html_response, res_to_error,
};
use crate::components::{
    HTPage, MaybeFillCheckbox, MaybeFillOption, MaybeFillTextArea, maybe_fill_value,
};
use crate::hyper;
use crate::lang;
use crate::resp_types::{RespAdminFederationHealth, RespInstanceInfo};
use crate::util::safe_href;
use render::Render;
use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::Arc;

const ADMIN_DIAGNOSTIC_SUMMARY_CHARS: usize = 180;
const ADMIN_BOOLEAN_SITE_SETTINGS: &[&str] = &[
    "cleanup_remote_posts_enabled",
    "cleanup_preview_posts_enabled",
    "cleanup_deleted_remote_communities_enabled",
    "cleanup_unfollowed_remote_communities_enabled",
    "cleanup_remote_interactions_enabled",
    "cleanup_notifications_enabled",
    "cleanup_failed_inbox_task_payloads_enabled",
];
const ADMIN_NUMERIC_SITE_SETTINGS: &[&str] = &[
    "cleanup_remote_post_retention_days",
    "cleanup_preview_post_retention_hours",
    "cleanup_notification_retention_days",
    "cleanup_failed_inbox_task_payload_retention_days",
    "cleanup_completed_task_retention_days",
    "cleanup_failed_task_retention_days",
    "cleanup_failed_inbox_task_payload_compaction_hours",
    "discovery_enqueue_limit",
    "discovery_refresh_interval_hours",
];

fn truncate_admin_diagnostic_summary(value: &str) -> String {
    let mut output = String::new();

    for (index, ch) in value.chars().enumerate() {
        if index >= ADMIN_DIAGNOSTIC_SUMMARY_CHARS {
            output.push_str("...");
            return output;
        }

        output.push(ch);
    }

    output
}

fn collapse_admin_diagnostic_text(value: &str) -> String {
    let value = unwrap_admin_diagnostic_debug_string(value);
    let value = value
        .replace("\\r\\n", " ")
        .replace("\\n", " ")
        .replace("\\r", " ")
        .replace("\\t", " ")
        .replace(['\r', '\n', '\t'], " ");

    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned()
}

fn unwrap_admin_diagnostic_debug_string(value: &str) -> String {
    let mut current = value.trim().to_owned();

    for _ in 0..3 {
        let trimmed = current.trim();

        if !(trimmed.starts_with("InternalStr(")
            || trimmed.starts_with("InternalStrStatic(")
            || trimmed.starts_with("Internal(Error("))
        {
            break;
        }

        let Some(first_quote) = trimmed.find('"') else {
            break;
        };
        let Some(last_quote) = trimmed.rfind('"') else {
            break;
        };

        if first_quote >= last_quote {
            break;
        }

        let quoted = &trimmed[first_quote..=last_quote];
        current = match serde_json::from_str::<String>(quoted) {
            Ok(decoded) => decoded,
            Err(_) => quoted.trim_matches('"').to_owned(),
        };
    }

    current
}

fn admin_diagnostic_summary(value: &str) -> String {
    let collapsed = collapse_admin_diagnostic_text(value);
    let lower = collapsed.to_ascii_lowercase();

    let summary = if lower.contains("domain_banned") || lower.contains("domain_blocked") {
        "Remote reported an explicit domain block"
    } else if lower.contains("domain") && lower.contains(" is blocked") {
        "Remote returned generic domain-block text"
    } else if lower.contains("fetch limit is > 50") {
        "Remote fetch limit is lower than requested"
    } else if lower.contains("remote request timed out")
        || lower.contains("remote response timed out")
        || lower.contains("timeout")
    {
        "Remote request timed out"
    } else if lower.contains("certificate verify failed") {
        "TLS certificate verification failed"
    } else if lower.contains("dns lookup failed")
        || lower.contains("dns error")
        || lower.contains("failed to lookup address")
        || lower.contains("failed to resolve")
    {
        "DNS lookup failed"
    } else if lower.contains("502 bad gateway") {
        "Remote returned 502 Bad Gateway"
    } else if lower.contains("instance_is_private") || lower.contains("instance is private") {
        "Instance is private"
    } else if lower.contains("route not found") || lower.contains("not found") {
        "Remote route not found"
    } else if lower.contains("connection refused") {
        "Remote connection refused"
    } else if lower.contains("no route to host") {
        "No route to host"
    } else if lower.contains("anubis") || lower.contains("oh noes") {
        "Remote returned a bot challenge page"
    } else if lower.contains("forbidden") {
        "Remote returned Forbidden"
    } else if lower.contains("no eligible remote post") {
        "No eligible remote post was available for probing"
    } else if lower.contains("unknown")
        && lower.contains("error in remote response")
        && lower.contains("message")
    {
        if lower.contains("\"message\":\"\"") || lower.contains("\"message\": \"\"") {
            "Remote returned an unknown error without a message"
        } else {
            "Remote returned an unknown error"
        }
    } else if lower.contains("eof while parsing a value") {
        "Remote returned incomplete JSON"
    } else if lower.contains("unknown content type found for activity") {
        "Remote returned an unsupported ActivityPub content type"
    } else {
        return truncate_admin_diagnostic_summary(&collapsed);
    };

    summary.to_owned()
}

fn admin_failure_category_label(value: Option<&str>) -> &'static str {
    match value {
        Some("domain_block") => "domain block",
        Some("user_or_community_ban") => "user/community ban",
        Some("timeout") => "timeout",
        Some("dns") => "DNS",
        Some("tls") => "TLS",
        Some("bot_challenge") => "bot challenge",
        Some("private") => "private",
        Some("no_probe_target") => "no probe target",
        Some("unsupported_activitypub") => "unsupported ActivityPub",
        Some("not_found") => "not found",
        Some("remote_5xx") => "remote 5xx",
        Some("connection") => "connection",
        Some("bad_remote_response") => "bad remote response",
        Some("suppressed") => "suppressed",
        Some("other") => "other",
        Some(_) => "uncategorized",
        None => "none",
    }
}

fn admin_catalog_status_label(value: Option<&str>) -> &'static str {
    match value {
        Some("useful_recent") => "useful, recent catalog",
        Some("useful_stale") => "useful, stale catalog",
        Some("verified_no_useful_catalog") => "verified, no useful catalog",
        Some("known_only") => "known only",
        Some("inactive") => "inactive",
        Some("suppressed") => "suppressed",
        Some(_) => "uncategorized",
        None => "unknown",
    }
}

fn admin_followed_community_health_label(value: &str) -> &'static str {
    match value {
        "missing_host_profile" => "missing host profile",
        "suppressed_host" => "host suppressed",
        "inactive_host" => "host inactive",
        "no_visible_posts" => "no visible posts",
        "stale_90d" => "no posts in 90 days",
        "stale_30d" => "no posts in 30 days",
        "catalog_stale" => "catalog stale",
        "ok" => "ok",
        _ => "unknown",
    }
}

fn admin_bytes_label(bytes: i64) -> String {
    let units = ["B", "KiB", "MiB", "GiB"];
    let bytes = u64::try_from(bytes).unwrap_or(0);
    let mut divisor = 1_u64;
    let mut unit_index = 0_usize;

    while unit_index + 1 < units.len() {
        let Some(next_divisor) = divisor.checked_mul(1024) else {
            break;
        };

        if bytes < next_divisor {
            break;
        }

        divisor = next_divisor;
        unit_index += 1;
    }

    let unit = units[unit_index];
    if unit_index == 0 {
        format!("{bytes} {unit}")
    } else {
        let mut whole = bytes / divisor;
        let remainder = bytes % divisor;
        let mut decimal = ((remainder * 10) + (divisor / 2)) / divisor;

        if decimal == 10 {
            whole = whole.saturating_add(1);
            decimal = 0;
        }

        format!("{whole}.{decimal} {unit}")
    }
}

struct AdminDiagnostic<'a> {
    value: Option<&'a str>,
}

impl render::Render for AdminDiagnostic<'_> {
    fn render_into<W: std::fmt::Write + ?Sized>(self, w: &mut W) -> std::fmt::Result {
        let Some(value) = self.value.map(str::trim).filter(|value| !value.is_empty()) else {
            return Ok(());
        };

        let summary = admin_diagnostic_summary(value);
        let collapsed = collapse_admin_diagnostic_text(value);

        if collapsed == summary {
            render::rsx! {
                <span class={"adminDiagnosticText"}>{summary.as_str()}</span>
            }
            .render_into(w)
        } else {
            render::rsx! {
                <details class={"adminDiagnostic"}>
                    <summary>{summary.as_str()}</summary>
                    <pre class={"adminDiagnosticRaw"}>{collapsed.as_str()}</pre>
                </details>
            }
            .render_into(w)
        }
    }
}

/*
    The administration dashboard is deliberately kept as one server-rendered
    page so related operator controls stay together. The render macro expands
    that page into a large typed tree, and the stricter stack-frame lint counts
    the generated template state even though the route renders it immediately.
*/
#[allow(clippy::large_stack_frames)]
async fn page_administration(
    _params: (),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let cookies = get_cookie_map_for_req(&req)?;
    let lang = crate::get_lang_for_req(&req);

    let base_data =
        fetch_base_data(&ctx.backend_host, &ctx.http_client, req.headers(), &cookies).await?;

    let title = lang.tr(&lang::ADMINISTRATION);

    if !base_data.is_site_admin() {
        return Ok(html_response(render::html! {
            <HTPage base_data={&base_data} lang={&lang} title={&title}>
                <h1>{title.as_ref()}</h1>
                <div class={"errorBox"}>
                    {lang.tr(&lang::not_site_admin())}
                </div>
            </HTPage>
        }));
    }

    let api_res = res_to_error(
        ctx.http_client
            .get(
                format!("{}/api/unstable/instance", ctx.backend_host)
                    .try_into()
                    .unwrap(),
            )
            .await?,
    )
    .await?;
    let api_res = crate::read_body_with_timeout(api_res.into_body()).await?;
    let api_res: RespInstanceInfo = serde_json::from_slice(&api_res)?;

    let federation_res = res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::get(format!(
                    "{}/api/unstable/instance/federation",
                    ctx.backend_host,
                ))
                .body(hyper::Body::default())?,
                req.headers(),
                &cookies,
            )?)
            .await?,
    )
    .await?;
    let federation_res = crate::read_body_with_timeout(federation_res.into_body()).await?;
    let federation_res: RespAdminFederationHealth = serde_json::from_slice(&federation_res)?;

    Ok(html_response(render::html! {
        <HTPage base_data={&base_data} lang={&lang} title={&title}>
            <h1>{title.as_ref()}</h1>
            <a href={"/administration/edit"}>{lang.tr(&lang::administration_edit())}</a>
            <ul>
                <li>
                    {"Site name: "}
                    <strong>{api_res.site_name.as_ref()}</strong>
                </li>
                <li>
                    {"Site logo: "}
                    {
                        api_res.site_logo.as_ref().map(|logo| {
                            render::rsx! {
                                <img src={safe_href(logo.url.as_ref()).unwrap_or("")} class={"siteLogoPreview"} alt={""} />
                            }
                        })
                    }
                    {
                        api_res.site_logo.is_none().then_some({
                            render::rsx! {
                                <span>{"none"}</span>
                            }
                        })
                    }
                </li>
                <li>
                    {
                        lang::TrElements::new(
                            lang.tr(&lang::administration_signup_allowed(lang::LangPlaceholder(0))),
                            |id, w| {
                                match id {
                                    0 => render::rsx! {
                                        <strong>{lang.tr(if api_res.signup_allowed {
                                            &lang::ALLOWED_TRUE
                                        } else {
                                            &lang::ALLOWED_FALSE
                                        })}</strong>
                                    }.render_into(w),
                                    _ => unreachable!(),
                                }
                            }
                        )
                    }
                </li>
                <li>
                    {
                        lang::TrElements::new(
                            lang.tr(&lang::administration_invitations_enabled(lang::LangPlaceholder(0))),
                            |id, w| {
                                match id {
                                    0 => render::rsx! {
                                        <strong>{lang.tr(if api_res.invitations_enabled {
                                            &lang::ENABLED_TRUE
                                        } else {
                                            &lang::ENABLED_FALSE
                                        })}</strong>
                                    }.render_into(w),
                                    _ => unreachable!(),
                                }
                            }
                        )
                    }
                    {
                        if api_res.invitations_enabled {
                            Some(render::rsx! {
                                <ul>
                                    <li>
                                        {lang.tr(&lang::ADMINISTRATION_INVITATION_CREATION_REQUIREMENT)}{" "}
                                        <strong>{lang.tr(match api_res.invitation_creation_requirement.as_deref() {
                                            None => &lang::REQUIREMENT_NONE,
                                            Some("site_admin") => &lang::REQUIREMENT_SITE_ADMIN,
                                            Some(_) => &lang::UNKNOWN,
                                        })}</strong>
                                    </li>
                                </ul>
                            })
                        } else {
                            None
                        }
                    }
                </li>
                <li>
                    {lang.tr(&lang::ADMINISTRATION_COMMUNITY_CREATION_REQUIREMENT)}{" "}
                    <strong>{lang.tr(match api_res.community_creation_requirement.as_deref() {
                        None => &lang::REQUIREMENT_NONE,
                        Some("site_admin") => &lang::REQUIREMENT_SITE_ADMIN,
                        Some(_) => &lang::UNKNOWN,
                    })}</strong>
                </li>
                <li>
                    {"Remote post purge: "}
                    <strong>{lang.tr(if api_res.cleanup_remote_posts_enabled {
                        &lang::ENABLED_TRUE
                    } else {
                        &lang::ENABLED_FALSE
                    })}</strong>
                    {" after "}
                    <strong>{api_res.cleanup_remote_post_retention_days.to_string()}</strong>
                    {" days"}
                </li>
                <li>
                    {"Preview post cache purge: "}
                    <strong>{lang.tr(if api_res.cleanup_preview_posts_enabled {
                        &lang::ENABLED_TRUE
                    } else {
                        &lang::ENABLED_FALSE
                    })}</strong>
                    {" after "}
                    <strong>{api_res.cleanup_preview_post_retention_hours.to_string()}</strong>
                    {" hours"}
                </li>
                <li>
                    {"Deleted remote community purge: "}
                    <strong>{lang.tr(if api_res.cleanup_deleted_remote_communities_enabled {
                        &lang::ENABLED_TRUE
                    } else {
                        &lang::ENABLED_FALSE
                    })}</strong>
                </li>
                <li>
                    {"Zero-follower remote community purge: "}
                    <strong>{lang.tr(if api_res.cleanup_unfollowed_remote_communities_enabled {
                        &lang::ENABLED_TRUE
                    } else {
                        &lang::ENABLED_FALSE
                    })}</strong>
                </li>
                <li>
                    {"Remote like cleanup: "}
                    <strong>{lang.tr(if api_res.cleanup_remote_interactions_enabled {
                        &lang::ENABLED_TRUE
                    } else {
                        &lang::ENABLED_FALSE
                    })}</strong>
                </li>
                <li>
                    {"Notification cleanup: "}
                    <strong>{lang.tr(if api_res.cleanup_notifications_enabled {
                        &lang::ENABLED_TRUE
                    } else {
                        &lang::ENABLED_FALSE
                    })}</strong>
                    {" after "}
                    <strong>{api_res.cleanup_notification_retention_days.to_string()}</strong>
                    {" days"}
                </li>
                <li>
                    {"Task cleanup: completed rows kept "}
                    <strong>{api_res.cleanup_completed_task_retention_days.to_string()}</strong>
                    {" days, failed rows kept "}
                    <strong>{api_res.cleanup_failed_task_retention_days.to_string()}</strong>
                    {" days"}
                </li>
                <li>
                    {"Failed inbox payload compaction: "}
                    <strong>{lang.tr(if api_res.cleanup_failed_inbox_task_payloads_enabled {
                        &lang::ENABLED_TRUE
                    } else {
                        &lang::ENABLED_FALSE
                    })}</strong>
                    {" after "}
                    <strong>{api_res.cleanup_failed_inbox_task_payload_compaction_hours.to_string()}</strong>
                    {" hours; failed inbox rows kept "}
                    <strong>{api_res.cleanup_failed_inbox_task_payload_retention_days.to_string()}</strong>
                    {" days"}
                </li>
                <li>
                    {"Community discovery: "}
                    <strong>{api_res.discovery_enqueue_limit.to_string()}</strong>
                    {" hosts per scheduler pass, healthy hosts refresh every "}
                    <strong>{api_res.discovery_refresh_interval_hours.to_string()}</strong>
                    {" hours"}
                </li>
            </ul>
            <h2>{"Federation health"}</h2>
            <ul>
                <li>
                    {"Known hosts: "}
                    <strong>{federation_res.summary.discovery_servers_total.to_string()}</strong>
                    {" total, "}
                    <strong>{federation_res.summary.discovery_servers_active.to_string()}</strong>
                    {" active, "}
                    <strong>{federation_res.summary.discovery_servers_inactive.to_string()}</strong>
                    {" inactive, "}
                    <strong>{federation_res.summary.discovery_servers_suppressed.to_string()}</strong>
                    {" suppressed"}
                </li>
                <li>
                    {"Community discovery hosts: "}
                    <strong>{federation_res.summary.discovery_servers_useful_sources.to_string()}</strong>
                    {" useful, "}
                    <strong>{federation_res.summary.discovery_servers_known_only.to_string()}</strong>
                    {" known only, "}
                    <strong>{federation_res.summary.discovery_servers_due.to_string()}</strong>
                    {" due for refresh"}
                </li>
                <li>
                    {"Interaction probes: "}
                    <strong>{federation_res.summary.discovery_servers_probe_success.to_string()}</strong>
                    {" hosts have passed at least one empirical probe"}
                </li>
                <li>
                    {"Task queue: "}
                    <strong>{federation_res.summary.task_pending_total.to_string()}</strong>
                    {" pending, "}
                    <strong>{federation_res.summary.task_running_total.to_string()}</strong>
                    {" running, "}
                    <strong>{federation_res.summary.task_failed_total.to_string()}</strong>
                    {" failed, "}
                    <strong>{federation_res.summary.task_completed_total.to_string()}</strong>
                    {" completed; table "}
                    <strong>{admin_bytes_label(federation_res.summary.task_table_bytes)}</strong>
                    {
                        federation_res.summary.task_oldest_pending.as_deref().map(|oldest| {
                            render::rsx! {
                                <>
                                    {"; oldest pending "}
                                    <strong>{oldest}</strong>
                                </>
                            }
                        })
                    }
                    <br />
                    <small>
                        {"pending lanes: outbound "}
                        {federation_res.summary.task_pending_outbound.to_string()}
                        {", inbox "}
                        {federation_res.summary.task_pending_inbox.to_string()}
                        {", discovery "}
                        {federation_res.summary.task_pending_discovery.to_string()}
                        {", preview "}
                        {federation_res.summary.task_pending_preview.to_string()}
                        {", readback "}
                        {federation_res.summary.task_pending_readback.to_string()}
                    </small>
                </li>
                <li>
                    {"Discovered communities: "}
                    <strong>{federation_res.summary.discovered_communities_total.to_string()}</strong>
                    {" total, "}
                    <strong>{federation_res.summary.discovered_communities_active.to_string()}</strong>
                    {" active, "}
                    <strong>{federation_res.summary.discovered_communities_with_posts.to_string()}</strong>
                    {" active with posts, "}
                    <strong>{federation_res.summary.discovered_communities_visible.to_string()}</strong>
                    {" visible useful rows"}
                </li>
                <li>
                    {"Actor platform profiles: "}
                    <strong>{federation_res.summary.actor_target_profiles_total.to_string()}</strong>
                </li>
                <li>
                    {"Blocked AP ids: "}
                    <strong>{federation_res.summary.blocked_ap_ids_total.to_string()}</strong>
                </li>
                <li>
                    {"Hidden communities: "}
                    <strong>{federation_res.summary.server_suppressed_communities_total.to_string()}</strong>
                    {" server scoped, "}
                    <strong>{federation_res.summary.user_suppressed_communities_total.to_string()}</strong>
                    {" user scoped"}
                </li>
                <li>
                    {"Federation events: "}
                    <strong>{federation_res.summary.federation_events_total.to_string()}</strong>
                </li>
            </ul>
            <h3>{"Followed community health"}</h3>
            <table class={"adminFederationTable"}>
                <thead>
                    <tr>
                        <th>{"Community"}</th>
                        <th>{"Host"}</th>
                        <th>{"Health"}</th>
                        <th>{"Posts"}</th>
                        <th>{"Catalog"}</th>
                        <th>{"Latest issue"}</th>
                    </tr>
                </thead>
                <tbody>
                    {
                        if federation_res.followed_community_health.is_empty() {
                            vec![render::rsx! {
                                <tr>
                                    <td>
                                        <a href={String::new()}>{"None"}</a>
                                        <br />
                                        <small>{""}</small>
                                    </td>
                                    <td>
                                        {""}
                                        <br />
                                        <small>{""}</small>
                                    </td>
                                    <td>
                                        {""}
                                        <br />
                                        <small>{String::new()}</small>
                                    </td>
                                    <td>
                                        {String::new()}
                                        <br />
                                        <small>{String::new()}</small>
                                    </td>
                                    <td>
                                        <small>{String::new()}</small>
                                        <br />
                                        <small>{String::new()}</small>
                                    </td>
                                    <td><AdminDiagnostic value={None} /></td>
                                </tr>
                            }]
                        } else {
                            federation_res.followed_community_health.iter().map(|community| {
                                let href = format!("/communities/{}", community.community_id);
                                let host_status = match community.host_active {
                                    Some(true) => "active",
                                    Some(false) => "inactive",
                                    None => "unknown",
                                };
                                let host_failures = community.host_failed_checks.unwrap_or(0);
                                let health_detail = community.suppressed_reason.as_deref()
                                    .or(community.latest_error.as_deref());
                                let post_counts = format!(
                                    "{} visible, {} remote",
                                    community.visible_posts,
                                    community.remote_post_count,
                                );
                                let last_post = community.last_post.as_deref().unwrap_or("never");
                                let catalog_seen = community.catalog_last_seen.as_deref()
                                    .unwrap_or("never");

                                render::rsx! {
                                    <tr>
                                        <td>
                                            <a href={href}>{community.community_name.as_str()}</a>
                                            <br />
                                            <small>{community.community_ap_id.as_deref().unwrap_or("")}</small>
                                        </td>
                                        <td>
                                            {community.host.as_str()}
                                            <br />
                                            <small>{community.software.as_deref().unwrap_or("")}</small>
                                        </td>
                                        <td>
                                            {admin_followed_community_health_label(community.health_status.as_str())}
                                            <br />
                                            <small>{format!("{host_status}; {host_failures} host failures")}</small>
                                        </td>
                                        <td>
                                            {post_counts}
                                            <br />
                                            <small>{format!("last post {last_post}; {} local followers", community.local_followers)}</small>
                                        </td>
                                        <td>
                                            <small>{format!("last seen {catalog_seen}")}</small>
                                            <br />
                                            <small>{format!("last success {}", community.last_success.as_deref().unwrap_or("never"))}</small>
                                        </td>
                                        <td><AdminDiagnostic value={health_detail} /></td>
                                    </tr>
                                }
                            }).collect::<Vec<_>>()
                        }
                    }
                </tbody>
            </table>
            <h3>{"Recent federation events"}</h3>
            <table class={"adminFederationTable"}>
                <thead>
                    <tr>
                        <th>{"Time"}</th>
                        <th>{"Direction"}</th>
                        <th>{"Action"}</th>
                        <th>{"Status"}</th>
                        <th>{"Host"}</th>
                        <th>{"Object"}</th>
                        <th>{"Error"}</th>
                    </tr>
                </thead>
                <tbody>
                    {
                        if federation_res.recent_events.is_empty() {
                            vec![render::rsx! {
                                <tr>
                                    <td>{""}</td>
                                    <td>{"None"}</td>
                                    <td>{""}</td>
                                    <td>{""}</td>
                                    <td>{""}</td>
                                    <td><small>{""}</small></td>
                                    <td><AdminDiagnostic value={None} /></td>
                                </tr>
                            }]
                        } else {
                            federation_res.recent_events.iter().map(|event| {
                                let object = event.object_ap_id.as_deref()
                                    .or(event.target_ap_id.as_deref())
                                    .or(event.actor_ap_id.as_deref())
                                    .unwrap_or("");
                                let error = event.error_text.as_deref()
                                    .or(event.error_class.as_deref());

                                render::rsx! {
                                    <tr>
                                        <td>{event.created_at.as_str()}</td>
                                        <td>{event.direction.as_str()}</td>
                                        <td>{event.action.as_str()}</td>
                                        <td>{event.status.as_str()}</td>
                                        <td>{event.host.as_deref().unwrap_or("")}</td>
                                        <td><small>{object}</small></td>
                                        <td><AdminDiagnostic value={error} /></td>
                                    </tr>
                                }
                            }).collect::<Vec<_>>()
                        }
                    }
                </tbody>
            </table>
            <h3>{"Replayable failed tasks"}</h3>
            <table class={"adminFederationTable"}>
                <thead>
                    <tr>
                        <th>{"Task"}</th>
                        <th>{"Kind"}</th>
                        <th>{"Attempts"}</th>
                        <th>{"Attempted"}</th>
                        <th>{"Error"}</th>
                        <th>{"Action"}</th>
                    </tr>
                </thead>
                <tbody>
                    {
                        if federation_res.replayable_failed_tasks.is_empty() {
                            vec![render::rsx! {
                                <tr>
                                    <td>{"None".to_owned()}</td>
                                    <td>{""}</td>
                                    <td>{String::new()}</td>
                                    <td>{""}</td>
                                    <td><AdminDiagnostic value={None} /></td>
                                    <td>
                                        <form method={"POST"} action={String::new()}>
                                            <button type={"submit"}>{""}</button>
                                        </form>
                                    </td>
                                </tr>
                            }]
                        } else {
                            federation_res.replayable_failed_tasks.iter().map(|task| {
                                let action = format!("/administration/federation/tasks/{}/retry", task.id);

                                render::rsx! {
                                    <tr>
                                        <td>{task.id.to_string()}</td>
                                        <td>{task.kind.as_str()}</td>
                                        <td>{format!("{}/{}", task.attempts, task.max_attempts)}</td>
                                        <td>{task.attempted_at.as_deref().unwrap_or(task.created_at.as_str())}</td>
                                        <td><AdminDiagnostic value={task.latest_error.as_deref()} /></td>
                                        <td>
                                            <form method={"POST"} action={action}>
                                                <button type={"submit"}>{"Retry"}</button>
                                            </form>
                                        </td>
                                    </tr>
                                }
                            }).collect::<Vec<_>>()
                        }
                    }
                </tbody>
            </table>
            <h3>{"Host capability profiles"}</h3>
            <table class={"adminFederationTable"}>
                <thead>
                    <tr>
                        <th>{"Host"}</th>
                        <th>{"Software"}</th>
                        <th>{"Health"}</th>
                        <th>{"Profile origin"}</th>
                        <th>{"Communities"}</th>
                        <th>{"Actor profiles"}</th>
                        <th>{"Recent events"}</th>
                    </tr>
                </thead>
                <tbody>
                    {
                        if federation_res.host_profiles.is_empty() {
                            vec![render::rsx! {
                                <tr>
                                    <td>{"None"}</td>
                                    <td>{""}</td>
                                    <td>
                                        {"inactive"}
                                        {"; "}
                                        {String::new()}
                                        {" failures"}
                                        <br />
                                        <AdminDiagnostic value={None} />
                                    </td>
                                    <td>
                                        {""}
                                        <br />
                                        <small>{String::new()}</small>
                                    </td>
                                    <td>{String::new()}</td>
                                    <td>{String::new()}</td>
                                    <td>{String::new()}</td>
                                </tr>
                            }]
                        } else {
                            federation_res.host_profiles.iter().map(|profile| {
                                let health = profile.suppressed_reason.as_deref()
                                    .or(profile.interaction_probe_latest_error.as_deref())
                                    .or(profile.latest_error.as_deref());
                                let communities = format!(
                                    "{} stored, {} followed, {} discovered active with posts",
                                    profile.communities_total,
                                    profile.followed_communities_total,
                                    profile.discovered_communities_with_posts,
                                );
                                let actors = format!(
                                    "{} total, {} high confidence",
                                    profile.actor_profiles_total,
                                    profile.high_confidence_actor_profiles_total,
                                );
                                let events = format!(
                                    "{} total, {} failed",
                                    profile.recent_events_total,
                                    profile.recent_failures_total,
                                );

                                render::rsx! {
                                    <tr>
                                        <td>{profile.host.as_str()}</td>
                                        <td>{profile.software.as_deref().unwrap_or("")}</td>
                                        <td>
                                            {if profile.active { "active" } else { "inactive" }}
                                            {"; "}
                                            {profile.failed_checks.to_string()}
                                            {" failures"}
                                            <br />
                                            <AdminDiagnostic value={health} />
                                        </td>
                                        <td>
                                            {admin_catalog_status_label(profile.catalog_status.as_deref())}
                                            <br />
                                            <small>{
                                                match profile.newest_community_seen.as_deref() {
                                                    Some(newest) => format!("last catalog row {newest}; {}", admin_failure_category_label(profile.failure_category.as_deref())),
                                                    None => admin_failure_category_label(profile.failure_category.as_deref()).to_owned(),
                                                }
                                            }</small>
                                        </td>
                                        <td>{communities}</td>
                                        <td>{actors}</td>
                                        <td>{events}</td>
                                    </tr>
                                }
                            }).collect::<Vec<_>>()
                        }
                    }
                </tbody>
            </table>
            <h3>{"Suppressed hosts"}</h3>
            <table class={"adminFederationTable"}>
                <thead>
                    <tr>
                        <th>{"Host"}</th>
                        <th>{"Software"}</th>
                        <th>{"Category"}</th>
                        <th>{"Reason"}</th>
                        <th>{"Probe error"}</th>
                    </tr>
                </thead>
                <tbody>
                    {
                        if federation_res.suppressed_servers.is_empty() {
                            vec![render::rsx! {
                                <tr>
                                    <td>{"None"}</td>
                                    <td>{""}</td>
                                    <td>{""}</td>
                                    <td><AdminDiagnostic value={None} /></td>
                                    <td><AdminDiagnostic value={None} /></td>
                                </tr>
                            }]
                        } else {
                            federation_res.suppressed_servers.iter().map(|server| {
                                render::rsx! {
                                    <tr>
                                        <td>{server.host.as_str()}</td>
                                        <td>{server.software.as_deref().unwrap_or("")}</td>
                                        <td>{admin_failure_category_label(server.failure_category.as_deref())}</td>
                                        <td>
                                            <AdminDiagnostic value={server.suppressed_reason.as_deref()} />
                                        </td>
                                        <td>
                                            <AdminDiagnostic value={server.interaction_probe_latest_error.as_deref()} />
                                        </td>
                                    </tr>
                                }
                            }).collect::<Vec<_>>()
                        }
                    }
                </tbody>
            </table>
            <h3>{"Failing hosts"}</h3>
            <table class={"adminFederationTable"}>
                <thead>
                    <tr>
                        <th>{"Host"}</th>
                        <th>{"Software"}</th>
                        <th>{"Active"}</th>
                        <th>{"Failures"}</th>
                        <th>{"Category"}</th>
                        <th>{"Latest error"}</th>
                    </tr>
                </thead>
                <tbody>
                    {
                        if federation_res.failing_servers.is_empty() {
                            vec![render::rsx! {
                                <tr>
                                    <td>{"None"}</td>
                                    <td>{""}</td>
                                    <td>{""}</td>
                                    <td>{String::new()}</td>
                                    <td>{""}</td>
                                    <td><AdminDiagnostic value={None} /></td>
                                </tr>
                            }]
                        } else {
                            federation_res.failing_servers.iter().map(|server| {
                                render::rsx! {
                                    <tr>
                                        <td>{server.host.as_str()}</td>
                                        <td>{server.software.as_deref().unwrap_or("")}</td>
                                        <td>{if server.active { "yes" } else { "no" }}</td>
                                        <td>{server.failed_checks.to_string()}</td>
                                        <td>{admin_failure_category_label(server.failure_category.as_deref())}</td>
                                        <td>
                                            <AdminDiagnostic value={server.latest_error.as_deref()
                                                .or(server.interaction_probe_latest_error.as_deref())} />
                                        </td>
                                    </tr>
                                }
                            }).collect::<Vec<_>>()
                        }
                    }
                </tbody>
            </table>
            <h3>{"Blocked AP ids"}</h3>
            <ul>
                {
                    if federation_res.blocked_ap_ids.is_empty() {
                        vec![render::rsx! {
                            <li>{"None"}</li>
                        }]
                    } else {
                        federation_res.blocked_ap_ids.iter().map(|blocked| {
                            render::rsx! {
                                <li>{blocked.ap_id.as_str()}</li>
                            }
                        }).collect::<Vec<_>>()
                    }
                }
            </ul>
            <h3>{"Suppressed communities"}</h3>
            <table class={"adminFederationTable"}>
                <thead>
                    <tr>
                        <th>{"Scope"}</th>
                        <th>{"Community"}</th>
                        <th>{"User"}</th>
                        <th>{"Reason"}</th>
                    </tr>
                </thead>
                <tbody>
                    {
                        if federation_res.server_suppressed_communities.is_empty()
                            && federation_res.user_suppressed_communities.is_empty()
                        {
                            vec![render::rsx! {
                                <tr>
                                    <td>{"None"}</td>
                                    <td>{""}{" "}<small>{""}</small></td>
                                    <td>{""}{" "}<small>{""}</small></td>
                                    <td><AdminDiagnostic value={None} /></td>
                                </tr>
                            }]
                        } else {
                            let mut rows = Vec::new();

                            rows.extend(federation_res.server_suppressed_communities.iter().map(|community| {
                                render::rsx! {
                                    <tr>
                                        <td>{"server"}</td>
                                        <td>
                                            {community.community_name.as_str()}
                                            {" "}
                                            <small>{community.community_ap_id.as_deref().unwrap_or("")}</small>
                                        </td>
                                        <td>{""}{" "}<small>{""}</small></td>
                                        <td>
                                            <AdminDiagnostic value={Some(community.reason.as_str())} />
                                        </td>
                                    </tr>
                                }
                            }));

                            rows.extend(federation_res.user_suppressed_communities.iter().map(|community| {
                                render::rsx! {
                                    <tr>
                                        <td>{"user"}</td>
                                        <td>
                                            {community.community_name.as_str()}
                                            {" "}
                                            <small>{community.community_ap_id.as_deref().unwrap_or("")}</small>
                                        </td>
                                        <td>
                                            {community.username.as_str()}
                                            {" "}
                                            <small>{community.person_ap_id.as_deref().unwrap_or("")}</small>
                                        </td>
                                        <td>
                                            <AdminDiagnostic value={Some(community.reason.as_str())} />
                                        </td>
                                    </tr>
                                }
                            }));

                            rows
                        }
                    }
                </tbody>
            </table>
            <h3>{"Actor platform profiles"}</h3>
            <table class={"adminFederationTable"}>
                <thead>
                    <tr>
                        <th>{"Family"}</th>
                        <th>{"Target"}</th>
                        <th>{"Actor kind"}</th>
                        <th>{"Count"}</th>
                        <th>{"High confidence"}</th>
                    </tr>
                </thead>
                <tbody>
                    {
                        if federation_res.actor_profile_families.is_empty() {
                            vec![render::rsx! {
                                <tr>
                                    <td>{"None"}</td>
                                    <td>{""}</td>
                                    <td>{""}</td>
                                    <td>{String::new()}</td>
                                    <td>{String::new()}</td>
                                </tr>
                            }]
                        } else {
                            federation_res.actor_profile_families.iter().map(|family| {
                                render::rsx! {
                                    <tr>
                                        <td>{family.family.as_str()}</td>
                                        <td>{family.target.as_str()}</td>
                                        <td>{family.actor_kind.as_str()}</td>
                                        <td>{family.count.to_string()}</td>
                                        <td>{family.high_confidence_count.to_string()}</td>
                                    </tr>
                                }
                            }).collect::<Vec<_>>()
                        }
                    }
                </tbody>
            </table>
            <h3>{"Recent actor profiles"}</h3>
            <table class={"adminFederationTable"}>
                <thead>
                    <tr>
                        <th>{"Actor"}</th>
                        <th>{"Family"}</th>
                        <th>{"Target"}</th>
                        <th>{"Confidence"}</th>
                        <th>{"Endpoints"}</th>
                    </tr>
                </thead>
                <tbody>
                    {
                        if federation_res.recent_actor_profiles.is_empty() {
                            vec![render::rsx! {
                                <tr>
                                    <td>{"None"}</td>
                                    <td>{""}</td>
                                    <td>{""}</td>
                                    <td>{String::new()}</td>
                                    <td>{String::new()}</td>
                                </tr>
                            }]
                        } else {
                            federation_res.recent_actor_profiles.iter().map(|profile| {
                                let endpoints = [
                                    if profile.has_inbox {
                                        Some("inbox")
                                    } else {
                                        None
                                    },
                                    if profile.has_outbox {
                                        Some("outbox")
                                    } else {
                                        None
                                    },
                                    if profile.has_followers {
                                        Some("followers")
                                    } else {
                                        None
                                    },
                                    if profile.has_featured {
                                        Some("featured")
                                    } else {
                                        None
                                    },
                                ]
                                    .into_iter()
                                    .flatten()
                                    .collect::<Vec<_>>()
                                    .join(", ");

                                render::rsx! {
                                    <tr>
                                        <td>{profile.actor_ap_id.as_str()}</td>
                                        <td>{profile.family.as_str()}</td>
                                        <td>{profile.target.as_str()}</td>
                                        <td>{profile.confidence.to_string()}</td>
                                        <td>{endpoints}</td>
                                    </tr>
                                }
                            }).collect::<Vec<_>>()
                        }
                    }
                </tbody>
            </table>
        </HTPage>
    }))
}

fn administration_description_edit_value<'a>(
    description: crate::resp_types::Content<'a>,
) -> (Cow<'a, str>, &'static str) {
    match description.content_markdown {
        Some(content) => (content, "markdown"),
        None => match description.content_html {
            Some(content) => (content, "html"),
            None => (
                description.content_text.unwrap_or(Cow::Borrowed("")),
                "text",
            ),
        },
    }
}

async fn page_administration_edit(
    _params: (),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let cookies = get_cookie_map_for_req(&req)?;

    page_administration_edit_inner(req.headers(), &cookies, ctx, None, None).await
}

async fn page_administration_edit_inner(
    headers: &hyper::header::HeaderMap,
    cookies: &CookieMap<'_>,
    ctx: Arc<crate::RouteContext>,
    display_error: Option<String>,
    prev_values: Option<&HashMap<Cow<'_, str>, serde_json::Value>>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let lang = crate::get_lang_for_headers(headers);

    let base_data = fetch_base_data(&ctx.backend_host, &ctx.http_client, headers, cookies).await?;

    let title = lang.tr(&lang::ADMINISTRATION_EDIT);

    if !base_data.is_site_admin() {
        return Ok(html_response(render::html! {
            <HTPage base_data={&base_data} lang={&lang} title={&title}>
                <h1>{title.as_ref()}</h1>
                <div class={"errorBox"}>
                    {lang.tr(&lang::not_site_admin())}
                </div>
            </HTPage>
        }));
    }

    let api_res = res_to_error(
        ctx.http_client
            .get(
                format!("{}/api/unstable/instance", ctx.backend_host)
                    .try_into()
                    .unwrap(),
            )
            .await?,
    )
    .await?;
    let api_res = crate::read_body_with_timeout(api_res.into_body()).await?;
    let api_res: RespInstanceInfo = serde_json::from_slice(&api_res)?;

    let signup_allowed_value = Some(crate::bool_as_str(api_res.signup_allowed));
    let invitations_enabled_value = Some(crate::bool_as_str(api_res.invitations_enabled));
    let invitation_creation_requirement_value = Some(
        api_res
            .invitation_creation_requirement
            .as_deref()
            .unwrap_or(""),
    );
    let community_creation_requirement_value = Some(
        api_res
            .community_creation_requirement
            .as_deref()
            .unwrap_or(""),
    );
    let site_name_value = Some(api_res.site_name.as_ref());
    let remote_post_retention_days = api_res.cleanup_remote_post_retention_days.to_string();
    let preview_post_retention_hours = api_res.cleanup_preview_post_retention_hours.to_string();
    let notification_retention_days = api_res.cleanup_notification_retention_days.to_string();
    let failed_inbox_payload_retention_days = api_res
        .cleanup_failed_inbox_task_payload_retention_days
        .to_string();
    let completed_task_retention_days = api_res.cleanup_completed_task_retention_days.to_string();
    let failed_task_retention_days = api_res.cleanup_failed_task_retention_days.to_string();
    let failed_inbox_payload_compaction_hours = api_res
        .cleanup_failed_inbox_task_payload_compaction_hours
        .to_string();
    let discovery_enqueue_limit = api_res.discovery_enqueue_limit.to_string();
    let discovery_refresh_interval_hours = api_res.discovery_refresh_interval_hours.to_string();

    let (description_content, description_format) =
        administration_description_edit_value(api_res.description);

    Ok(html_response(render::html! {
        <HTPage base_data={&base_data} lang={&lang} title={&title}>
            <h1>{title.as_ref()}</h1>
            {
                display_error.map(|msg| {
                    render::rsx! {
                        <div class={"errorBox"}>{msg}</div>
                    }
                })
            }
            <form method={"POST"} action={"/administration/edit/logo"} enctype={"multipart/form-data"}>
                <fieldset>
                    <legend>{"Logo"}</legend>
                    {
                        api_res.site_logo.as_ref().map(|logo| {
                            render::rsx! {
                                <div>
                                    <img src={safe_href(logo.url.as_ref()).unwrap_or("")} class={"siteLogoPreview"} alt={""} />
                                </div>
                            }
                        })
                    }
                    <div>
                        <label>
                            {"Site logo"}<br />
                            <input type={"file"} name={"site_logo_media"} accept={"image/*"} />
                        </label>
                    </div>
                    <div>
                        <label>
                            <input type={"checkbox"} name={"remove_site_logo"} />
                            {" Remove logo"}
                        </label>
                    </div>
                    <button type={"submit"}>{"Upload logo"}</button>
                </fieldset>
            </form>
            <form method={"POST"} action={"/administration/edit/stylesheet"} enctype={"multipart/form-data"}>
                <fieldset>
                    <legend>{"Stylesheet"}</legend>
                    {
                        api_res.site_css.as_ref().map(|css| {
                            render::rsx! {
                                <p>
                                    {"Custom CSS is active: "}
                                    <a href={safe_href(css.url.as_ref()).unwrap_or("")}>{"view stylesheet"}</a>
                                </p>
                            }
                        })
                    }
                    {
                        api_res.site_css.is_none().then_some({
                            render::rsx! {
                                <p>{"Using bundled CSS."}</p>
                            }
                        })
                    }
                    <div>
                        <label>
                            {"Site CSS"}<br />
                            <input type={"file"} name={"site_css_media"} accept={"text/css,.css"} />
                        </label>
                    </div>
                    <div>
                        <label>
                            <input type={"checkbox"} name={"remove_site_css"} />
                            {" Remove custom CSS"}
                        </label>
                    </div>
                    <button type={"submit"}>{"Upload CSS"}</button>
                </fieldset>
            </form>
            <form method={"POST"} action={"/administration/edit/submit"}>
                <div>
                    <label>
                        {"Site name"}<br />
                        <input
                            type={"text"}
                            name={"site_name"}
                            value={maybe_fill_value(&prev_values, "site_name", site_name_value)}
                            maxlength={"80"}
                            required={""}
                        />
                    </label>
                </div>
                <div>
                    <label>
                        {lang.tr(&lang::administration_edit_signup_allowed())}<br />
                        <select name={"signup_allowed"}>
                            <MaybeFillOption value={"true"} values={&prev_values} default_value={signup_allowed_value} name={"signup_allowed"}>
                                {lang.tr(&lang::allowed_true())}
                            </MaybeFillOption>
                            <MaybeFillOption value={"false"} values={&prev_values} default_value={signup_allowed_value} name={"signup_allowed"}>
                                {lang.tr(&lang::allowed_false())}
                            </MaybeFillOption>
                        </select>
                    </label>
                </div>
                <div>
                    <label>
                        {lang.tr(&lang::administration_edit_invitations_enabled())}<br />
                        <select name={"invitations_enabled"}>
                            <MaybeFillOption value={"true"} values={&prev_values} default_value={invitations_enabled_value} name={"invitations_enabled"}>
                                {lang.tr(&lang::enabled_true())}
                            </MaybeFillOption>
                            <MaybeFillOption value={"false"} values={&prev_values} default_value={invitations_enabled_value} name={"invitations_enabled"}>
                                {lang.tr(&lang::enabled_false())}
                            </MaybeFillOption>
                        </select>
                    </label>
                </div>
                <div>
                    <label>
                        {lang.tr(&lang::administration_invitation_creation_requirement())}{":"}<br />
                        <select name={"invitation_creation_requirement"}>
                            <MaybeFillOption value={""} values={&prev_values} default_value={invitation_creation_requirement_value} name={"invitation_creation_requirement"}>
                                {lang.tr(&lang::requirement_none())}
                            </MaybeFillOption>
                            <MaybeFillOption value={"site_admin"} values={&prev_values} default_value={invitation_creation_requirement_value} name={"invitation_creation_requirement"}>
                                {lang.tr(&lang::requirement_site_admin())}
                            </MaybeFillOption>
                        </select>
                    </label>
                </div>
                <div>
                    <label>
                        {lang.tr(&lang::administration_community_creation_requirement())}{":"}<br />
                        <select name={"community_creation_requirement"}>
                            <MaybeFillOption value={""} values={&prev_values} default_value={community_creation_requirement_value} name={"community_creation_requirement"}>
                                {lang.tr(&lang::requirement_none())}
                            </MaybeFillOption>
                            <MaybeFillOption value={"site_admin"} values={&prev_values} default_value={community_creation_requirement_value} name={"community_creation_requirement"}>
                                {lang.tr(&lang::requirement_site_admin())}
                            </MaybeFillOption>
                        </select>
                    </label>
                </div>
                <fieldset>
                    <legend>{"Cleanup jobs"}</legend>
                    <div>
                        <label>
                            <MaybeFillCheckbox
                                values={&prev_values}
                                name={"cleanup_remote_posts_enabled"}
                                id={"cleanup_remote_posts_enabled"}
                                default={api_res.cleanup_remote_posts_enabled}
                            />
                            {" Purge old remote posts and deleted remote posts"}
                        </label>
                    </div>
                    <div>
                        <label>
                            {"Remote post retention days"}<br />
                            <input
                                type={"number"}
                                name={"cleanup_remote_post_retention_days"}
                                value={maybe_fill_value(&prev_values, "cleanup_remote_post_retention_days", Some(remote_post_retention_days.as_str()))}
                                min={"1"}
                                max={"3650"}
                                required={""}
                            />
                        </label>
                    </div>
                    <div>
                        <label>
                            <MaybeFillCheckbox
                                values={&prev_values}
                                name={"cleanup_preview_posts_enabled"}
                                id={"cleanup_preview_posts_enabled"}
                                default={api_res.cleanup_preview_posts_enabled}
                            />
                            {" Purge unfollowed preview posts"}
                        </label>
                    </div>
                    <div>
                        <label>
                            {"Preview post retention hours"}<br />
                            <input
                                type={"number"}
                                name={"cleanup_preview_post_retention_hours"}
                                value={maybe_fill_value(&prev_values, "cleanup_preview_post_retention_hours", Some(preview_post_retention_hours.as_str()))}
                                min={"1"}
                                max={"720"}
                                required={""}
                            />
                        </label>
                    </div>
                    <div>
                        <label>
                            <MaybeFillCheckbox
                                values={&prev_values}
                                name={"cleanup_deleted_remote_communities_enabled"}
                                id={"cleanup_deleted_remote_communities_enabled"}
                                default={api_res.cleanup_deleted_remote_communities_enabled}
                            />
                            {" Delete empty remote communities marked deleted"}
                        </label>
                    </div>
                    <div>
                        <label>
                            <MaybeFillCheckbox
                                values={&prev_values}
                                name={"cleanup_unfollowed_remote_communities_enabled"}
                                id={"cleanup_unfollowed_remote_communities_enabled"}
                                default={api_res.cleanup_unfollowed_remote_communities_enabled}
                            />
                            {" Delete empty remote communities with no local followers"}
                        </label>
                    </div>
                    <div>
                        <label>
                            <MaybeFillCheckbox
                                values={&prev_values}
                                name={"cleanup_remote_interactions_enabled"}
                                id={"cleanup_remote_interactions_enabled"}
                                default={api_res.cleanup_remote_interactions_enabled}
                            />
                            {" Purge old remote likes on purgeable remote posts and comments"}
                        </label>
                    </div>
                    <div>
                        <label>
                            <MaybeFillCheckbox
                                values={&prev_values}
                                name={"cleanup_notifications_enabled"}
                                id={"cleanup_notifications_enabled"}
                                default={api_res.cleanup_notifications_enabled}
                            />
                            {" Purge old read notifications"}
                        </label>
                    </div>
                    <div>
                        <label>
                            {"Notification retention days"}<br />
                            <input
                                type={"number"}
                                name={"cleanup_notification_retention_days"}
                                value={maybe_fill_value(&prev_values, "cleanup_notification_retention_days", Some(notification_retention_days.as_str()))}
                                min={"1"}
                                max={"3650"}
                                required={""}
                            />
                        </label>
                    </div>
                    <div>
                        <label>
                            <MaybeFillCheckbox
                                values={&prev_values}
                                name={"cleanup_failed_inbox_task_payloads_enabled"}
                                id={"cleanup_failed_inbox_task_payloads_enabled"}
                                default={api_res.cleanup_failed_inbox_task_payloads_enabled}
                            />
                            {" Discard failed inbox task payloads after permanent failure"}
                        </label>
                    </div>
                    <div>
                        <label>
                            {"Completed task retention days"}<br />
                            <input
                                type={"number"}
                                name={"cleanup_completed_task_retention_days"}
                                value={maybe_fill_value(&prev_values, "cleanup_completed_task_retention_days", Some(completed_task_retention_days.as_str()))}
                                min={"1"}
                                max={"30"}
                                required={""}
                            />
                        </label>
                    </div>
                    <div>
                        <label>
                            {"Failed task retention days"}<br />
                            <input
                                type={"number"}
                                name={"cleanup_failed_task_retention_days"}
                                value={maybe_fill_value(&prev_values, "cleanup_failed_task_retention_days", Some(failed_task_retention_days.as_str()))}
                                min={"1"}
                                max={"365"}
                                required={""}
                            />
                        </label>
                    </div>
                    <div>
                        <label>
                            {"Failed inbox payload compaction hours"}<br />
                            <input
                                type={"number"}
                                name={"cleanup_failed_inbox_task_payload_compaction_hours"}
                                value={maybe_fill_value(&prev_values, "cleanup_failed_inbox_task_payload_compaction_hours", Some(failed_inbox_payload_compaction_hours.as_str()))}
                                min={"1"}
                                max={"168"}
                                required={""}
                            />
                        </label>
                    </div>
                    <div>
                        <label>
                            {"Failed inbox task row retention days"}<br />
                            <input
                                type={"number"}
                                name={"cleanup_failed_inbox_task_payload_retention_days"}
                                value={maybe_fill_value(&prev_values, "cleanup_failed_inbox_task_payload_retention_days", Some(failed_inbox_payload_retention_days.as_str()))}
                                min={"1"}
                                max={"365"}
                                required={""}
                            />
                        </label>
                    </div>
                </fieldset>
                <fieldset>
                    <legend>{"Discovery jobs"}</legend>
                    <div>
                        <label>
                            {"Discovery batch size"}<br />
                            <input
                                type={"number"}
                                name={"discovery_enqueue_limit"}
                                value={maybe_fill_value(&prev_values, "discovery_enqueue_limit", Some(discovery_enqueue_limit.as_str()))}
                                min={"10"}
                                max={"500"}
                                required={""}
                            />
                        </label>
                    </div>
                    <div>
                        <label>
                            {"Healthy host refresh interval hours"}<br />
                            <input
                                type={"number"}
                                name={"discovery_refresh_interval_hours"}
                                value={maybe_fill_value(&prev_values, "discovery_refresh_interval_hours", Some(discovery_refresh_interval_hours.as_str()))}
                                min={"1"}
                                max={"168"}
                                required={""}
                            />
                        </label>
                    </div>
                </fieldset>
                <label>
                    {lang.tr(&lang::description())}
                    <br />
                    <MaybeFillTextArea values={&prev_values} name={"description"} default_value={Some(&description_content)} />
                    <br />
                    <select name={"description_format"}>
                        <MaybeFillOption value={"text"} values={&prev_values} default_value={Some(description_format)} name={"description_format"}>
                            {lang.tr(&lang::content_format_text())}
                        </MaybeFillOption>
                        <MaybeFillOption value={"markdown"} values={&prev_values} default_value={Some(description_format)} name={"description_format"}>
                            {lang.tr(&lang::content_format_markdown())}
                        </MaybeFillOption>
                        <MaybeFillOption value={"html"} values={&prev_values} default_value={Some(description_format)} name={"description_format"}>
                            {lang.tr(&lang::content_format_html())}
                        </MaybeFillOption>
                    </select>
                </label>
                <br />
                <br />
                <button type={"submit"}>{"Save"}</button>
            </form>
        </HTPage>
    }))
}

async fn patch_instance_from_admin(
    ctx: &crate::RouteContext,
    headers: &hyper::header::HeaderMap,
    cookies: &CookieMap<'_>,
    body: serde_json::Value,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::patch(format!("{}/api/unstable/instance", ctx.backend_host))
                    .body(serde_json::to_vec(&body)?.into())?,
                headers,
                cookies,
            )?)
            .await?,
    )
    .await
}

async fn handler_administration_logo_submit(
    _params: (),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (req_parts, body) = req.into_parts();

    let cookies = get_cookie_map_for_headers(&req_parts.headers)?;

    let content_type = req_parts
        .headers
        .get(hyper::header::CONTENT_TYPE)
        .ok_or_else(|| {
            crate::Error::InternalStr("missing content-type header in form submission".to_owned())
        })?;
    let content_type = std::str::from_utf8(content_type.as_ref())?;
    let boundary = multer::parse_boundary(content_type)?;

    let mut multipart = multer::Multipart::new(body, boundary);
    let mut remove_site_logo = false;
    let mut site_logo = None;

    loop {
        let field = multipart.next_field().await?;
        let field = match field {
            None => break,
            Some(field) => field,
        };

        let field_name = match field.name() {
            None => continue,
            Some(name) => name.to_owned(),
        };

        if field_name == "remove_site_logo" {
            remove_site_logo = true;
            continue;
        }

        if field_name == "site_logo_media" {
            use futures_util::StreamExt;

            let mut stream = field.peekable();
            let first_chunk = std::pin::Pin::new(&mut stream).peek().await;
            let is_empty = match first_chunk {
                None => true,
                Some(Ok(chunk)) => chunk.is_empty(),
                Some(Err(err)) => {
                    return Err(crate::Error::InternalStr(format!(
                        "failed parsing form: {err:?}"
                    )));
                }
            };

            if is_empty {
                continue;
            }

            let mime = match stream.get_ref().content_type() {
                None => {
                    return page_administration_edit_inner(
                        &req_parts.headers,
                        &cookies,
                        ctx,
                        Some("Missing Content-Type for logo upload".to_owned()),
                        None,
                    )
                    .await;
                }
                Some(mime) => mime,
            };

            let res = res_to_error(
                ctx.http_client
                    .request_upload(for_client(
                        hyper::Request::post(format!("{}/api/unstable/media", ctx.backend_host))
                            .header(hyper::header::CONTENT_TYPE, mime.as_ref())
                            .body(hyper::Body::wrap_stream(stream))?,
                        &req_parts.headers,
                        &cookies,
                    )?)
                    .await?,
            )
            .await;

            match res {
                Err(crate::Error::RemoteError((_, message))) => {
                    return page_administration_edit_inner(
                        &req_parts.headers,
                        &cookies,
                        ctx,
                        Some(message),
                        None,
                    )
                    .await;
                }
                Err(other) => return Err(other),
                Ok(res) => {
                    let res = crate::read_body_with_timeout(res.into_body()).await?;
                    let res: JustStringID = serde_json::from_slice(&res)?;

                    site_logo = Some(format!("local-media://{}", res.id));
                }
            }
        }
    }

    let body = if remove_site_logo {
        serde_json::json!({ "site_logo": null })
    } else if let Some(site_logo) = site_logo {
        serde_json::json!({ "site_logo": site_logo })
    } else {
        return page_administration_edit_inner(
            &req_parts.headers,
            &cookies,
            ctx,
            Some("Choose a logo image to upload".to_owned()),
            None,
        )
        .await;
    };

    match patch_instance_from_admin(&ctx, &req_parts.headers, &cookies, body).await {
        Err(crate::Error::RemoteError((_, message))) => {
            page_administration_edit_inner(&req_parts.headers, &cookies, ctx, Some(message), None)
                .await
        }
        Err(other) => Err(other),
        Ok(_) => Ok(hyper::Response::builder()
            .status(hyper::StatusCode::SEE_OTHER)
            .header(hyper::header::LOCATION, "/administration/edit")
            .body("Successfully edited.".into())?),
    }
}

async fn handler_administration_stylesheet_submit(
    _params: (),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (req_parts, body) = req.into_parts();

    let cookies = get_cookie_map_for_headers(&req_parts.headers)?;

    let content_type = req_parts
        .headers
        .get(hyper::header::CONTENT_TYPE)
        .ok_or_else(|| {
            crate::Error::InternalStr("missing content-type header in form submission".to_owned())
        })?;
    let content_type = std::str::from_utf8(content_type.as_ref())?;
    let boundary = multer::parse_boundary(content_type)?;

    let mut multipart = multer::Multipart::new(body, boundary);
    let mut remove_site_css = false;
    let mut uploaded = false;

    loop {
        let field = multipart.next_field().await?;
        let field = match field {
            None => break,
            Some(field) => field,
        };

        let field_name = match field.name() {
            None => continue,
            Some(name) => name.to_owned(),
        };

        if field_name == "remove_site_css" {
            remove_site_css = true;
            continue;
        }

        if field_name == "site_css_media" {
            use futures_util::StreamExt;

            let mut stream = field.peekable();
            let first_chunk = std::pin::Pin::new(&mut stream).peek().await;
            let is_empty = match first_chunk {
                None => true,
                Some(Ok(chunk)) => chunk.is_empty(),
                Some(Err(err)) => {
                    return Err(crate::Error::InternalStr(format!(
                        "failed parsing form: {err:?}"
                    )));
                }
            };

            if is_empty {
                continue;
            }

            let mime = match stream.get_ref().content_type() {
                None => {
                    return page_administration_edit_inner(
                        &req_parts.headers,
                        &cookies,
                        ctx,
                        Some("Missing Content-Type for CSS upload".to_owned()),
                        None,
                    )
                    .await;
                }
                Some(mime) => mime,
            };

            let res = res_to_error(
                ctx.http_client
                    .request_upload(for_client(
                        hyper::Request::post(format!(
                            "{}/api/unstable/instance/stylesheet",
                            ctx.backend_host,
                        ))
                        .header(hyper::header::CONTENT_TYPE, mime.as_ref())
                        .body(hyper::Body::wrap_stream(stream))?,
                        &req_parts.headers,
                        &cookies,
                    )?)
                    .await?,
            )
            .await;

            match res {
                Err(crate::Error::RemoteError((_, message))) => {
                    return page_administration_edit_inner(
                        &req_parts.headers,
                        &cookies,
                        ctx,
                        Some(message),
                        None,
                    )
                    .await;
                }
                Err(other) => return Err(other),
                Ok(_) => {
                    uploaded = true;
                }
            }
        }
    }

    if remove_site_css {
        match res_to_error(
            ctx.http_client
                .request(for_client(
                    hyper::Request::delete(format!(
                        "{}/api/unstable/instance/stylesheet",
                        ctx.backend_host,
                    ))
                    .body(hyper::Body::default())?,
                    &req_parts.headers,
                    &cookies,
                )?)
                .await?,
        )
        .await
        {
            Err(crate::Error::RemoteError((_, message))) => {
                return page_administration_edit_inner(
                    &req_parts.headers,
                    &cookies,
                    ctx,
                    Some(message),
                    None,
                )
                .await;
            }
            Err(other) => return Err(other),
            Ok(_) => {}
        }
    } else if !uploaded {
        return page_administration_edit_inner(
            &req_parts.headers,
            &cookies,
            ctx,
            Some("Choose a CSS file to upload".to_owned()),
            None,
        )
        .await;
    }

    Ok(hyper::Response::builder()
        .status(hyper::StatusCode::SEE_OTHER)
        .header(hyper::header::LOCATION, "/administration/edit")
        .body("Successfully edited.".into())?)
}

async fn handler_administration_edit_submit(
    _params: (),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (req_parts, body) = req.into_parts();

    let cookies = get_cookie_map_for_headers(&req_parts.headers)?;

    let body = crate::read_body_with_timeout(body).await?;
    let body_original: HashMap<Cow<'_, str>, serde_json::Value> =
        serde_urlencoded::from_bytes(&body)?;
    let mut body = body_original.clone();

    for key in ["signup_allowed", "invitations_enabled"] {
        body.insert(
            key.into(),
            body.get(key)
                .and_then(|x| x.as_str())
                .ok_or(crate::Error::InternalStrStatic(
                    "Failed to extract value in administration edit",
                ))?
                .parse()?,
        );
    }

    for &key in ADMIN_BOOLEAN_SITE_SETTINGS {
        body.insert(key.into(), body_original.contains_key(key).into());
    }

    for &key in ADMIN_NUMERIC_SITE_SETTINGS {
        let value = match body.get(key).and_then(|x| x.as_str()) {
            Some(value) => value,
            None => {
                return page_administration_edit_inner(
                    &req_parts.headers,
                    &cookies,
                    ctx,
                    Some(format!("Missing numeric value for {key}")),
                    Some(&body_original),
                )
                .await;
            }
        };

        let value: i32 = match value.parse() {
            Ok(value) => value,
            Err(_) => {
                return page_administration_edit_inner(
                    &req_parts.headers,
                    &cookies,
                    ctx,
                    Some(format!("Invalid numeric value for {key}")),
                    Some(&body_original),
                )
                .await;
            }
        };

        body.insert(key.into(), value.into());
    }

    for key in [
        "invitation_creation_requirement",
        "community_creation_requirement",
    ] {
        if body.get(key).and_then(|x| x.as_str()) == Some("") {
            body.insert(key.into(), serde_json::Value::Null);
        }
    }

    if let Some(content) = body.remove("description") {
        let content = content.as_str().ok_or(crate::Error::InternalStrStatic(
            "Failed to extract description in administration edit",
        ))?;

        let format = body.remove("description_format");
        let format = match format.as_ref().and_then(|x| x.as_str()) {
            Some(format) => format,
            None => {
                return Err(crate::Error::InternalStrStatic(
                    "Invalid or missing description format",
                ));
            }
        };

        body.insert(format!("description_{format}").into(), content.into());
    }

    let api_res = res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::patch(format!("{}/api/unstable/instance", ctx.backend_host))
                    .body(serde_json::to_vec(&body)?.into())?,
                &req_parts.headers,
                &cookies,
            )?)
            .await?,
    )
    .await;

    match api_res {
        Err(crate::Error::RemoteError((_, message))) => {
            page_administration_edit_inner(
                &req_parts.headers,
                &cookies,
                ctx,
                Some(message),
                Some(&body),
            )
            .await
        }
        Err(other) => Err(other),
        Ok(_) => Ok(hyper::Response::builder()
            .status(hyper::StatusCode::SEE_OTHER)
            .header(hyper::header::LOCATION, "/administration")
            .body("Successfully edited.".into())?),
    }
}

async fn handler_administration_federation_task_retry(
    params: (i64,),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (task_id,) = params;
    let cookies = get_cookie_map_for_req(&req)?;

    res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::post(format!(
                    "{}/api/unstable/instance/federation/tasks/{}/retry",
                    ctx.backend_host, task_id
                ))
                .body(hyper::Body::default())?,
                req.headers(),
                &cookies,
            )?)
            .await?,
    )
    .await?;

    Ok(hyper::Response::builder()
        .status(hyper::StatusCode::SEE_OTHER)
        .header(hyper::header::LOCATION, "/administration")
        .body("Task queued.".into())?)
}

pub fn route_administration() -> crate::RouteNode<()> {
    crate::RouteNode::new()
        .with_handler_async(hyper::Method::GET, page_administration)
        .with_child(
            "edit",
            crate::RouteNode::new()
                .with_handler_async(hyper::Method::GET, page_administration_edit)
                .with_child(
                    "logo",
                    crate::RouteNode::new().with_handler_async(
                        hyper::Method::POST,
                        handler_administration_logo_submit,
                    ),
                )
                .with_child(
                    "stylesheet",
                    crate::RouteNode::new().with_handler_async(
                        hyper::Method::POST,
                        handler_administration_stylesheet_submit,
                    ),
                )
                .with_child(
                    "submit",
                    crate::RouteNode::new().with_handler_async(
                        hyper::Method::POST,
                        handler_administration_edit_submit,
                    ),
                ),
        )
        .with_child(
            "federation",
            crate::RouteNode::new().with_child("tasks", {
                crate::RouteNode::new().with_child_parse::<i64, _>(
                    crate::RouteNode::new().with_child(
                        "retry",
                        crate::RouteNode::new().with_handler_async(
                            hyper::Method::POST,
                            handler_administration_federation_task_retry,
                        ),
                    ),
                )
            }),
        )
}

#[cfg(test)]
mod tests {
    #[test]
    fn admin_diagnostic_summary_identifies_common_remote_failures() {
        assert_eq!(
            super::admin_diagnostic_summary(
                "InternalStr(\"Error in remote response: {\\\"error\\\":\\\"unknown\\\",\\\"message\\\":\\\"Domain \\\\\\\"lotide.example\\\\\\\" is blocked\\\"}\")"
            ),
            "Remote returned generic domain-block text"
        );
        assert_eq!(
            super::admin_diagnostic_summary(
                "InternalStr(\"Error in remote response: {\\\"error\\\":\\\"domain_banned\\\"}\")"
            ),
            "Remote reported an explicit domain block"
        );
        assert_eq!(
            super::admin_diagnostic_summary("InternalStr(\"Error in remote response: Forbidden\")"),
            "Remote returned Forbidden"
        );
        assert_eq!(
            super::admin_diagnostic_summary("InternalStrStatic(\"Remote request timed out\")"),
            "Remote request timed out"
        );
        assert_eq!(
            super::admin_diagnostic_summary(
                "InternalStr(\"Error in remote response: {\\\"error\\\":\\\"unknown\\\",\\\"message\\\":\\\"\\\"}\")"
            ),
            "Remote returned an unknown error without a message"
        );
        assert_eq!(
            super::admin_diagnostic_summary(
                "InternalStr(\"lemmy-compatible failed: EOF while parsing a value\")"
            ),
            "Remote returned incomplete JSON"
        );
        assert_eq!(
            super::admin_diagnostic_summary("InternalStrStatic(\"DNS lookup failed\")"),
            "DNS lookup failed"
        );
        assert_eq!(
            super::admin_diagnostic_summary("<html><title>Oh noes!</title>Anubis</html>"),
            "Remote returned a bot challenge page"
        );
    }

    #[test]
    fn admin_diagnostic_summary_collapses_escaped_newlines() {
        assert_eq!(
            super::collapse_admin_diagnostic_text(
                "InternalStr(\"line one\\r\\nline two\")\nline three"
            ),
            "line one line two"
        );
        assert_eq!(
            super::collapse_admin_diagnostic_text(
                "Internal(Error(\"data did not match any variant\"))"
            ),
            "data did not match any variant"
        );
    }

    #[test]
    fn admin_diagnostic_summary_truncates_unknown_long_messages() {
        let long = "x".repeat(super::ADMIN_DIAGNOSTIC_SUMMARY_CHARS + 20);
        let summary = super::admin_diagnostic_summary(&long);

        assert!(summary.ends_with("..."));
        assert!(summary.len() < long.len());
    }

    #[test]
    fn admin_failure_category_labels_are_human_readable() {
        assert_eq!(
            super::admin_failure_category_label(Some("domain_block")),
            "domain block"
        );
        assert_eq!(super::admin_failure_category_label(Some("dns")), "DNS");
        assert_eq!(
            super::admin_failure_category_label(Some("bot_challenge")),
            "bot challenge"
        );
        assert_eq!(super::admin_failure_category_label(None), "none");
    }

    #[test]
    fn admin_catalog_status_labels_separate_fresh_and_stale_sources() {
        assert_eq!(
            super::admin_catalog_status_label(Some("useful_recent")),
            "useful, recent catalog"
        );
        assert_eq!(
            super::admin_catalog_status_label(Some("useful_stale")),
            "useful, stale catalog"
        );
        assert_eq!(
            super::admin_catalog_status_label(Some("verified_no_useful_catalog")),
            "verified, no useful catalog"
        );
    }

    #[test]
    fn admin_description_edit_value_tolerates_empty_backend_description() {
        let empty = crate::resp_types::Content::default();
        let text = crate::resp_types::Content {
            content_text: Some(std::borrow::Cow::Borrowed("plain")),
            ..Default::default()
        };
        let html = crate::resp_types::Content {
            content_html: Some(std::borrow::Cow::Borrowed("<p>html</p>")),
            ..Default::default()
        };
        let markdown = crate::resp_types::Content {
            content_markdown: Some(std::borrow::Cow::Borrowed("**markdown**")),
            ..Default::default()
        };

        assert_eq!(
            super::administration_description_edit_value(empty),
            ("".into(), "text")
        );
        assert_eq!(
            super::administration_description_edit_value(text),
            ("plain".into(), "text")
        );
        assert_eq!(
            super::administration_description_edit_value(html),
            ("<p>html</p>".into(), "html")
        );
        assert_eq!(
            super::administration_description_edit_value(markdown),
            ("**markdown**".into(), "markdown")
        );
    }

    #[test]
    fn admin_followed_community_health_labels_are_operator_readable() {
        assert_eq!(
            super::admin_followed_community_health_label("missing_host_profile"),
            "missing host profile"
        );
        assert_eq!(
            super::admin_followed_community_health_label("no_visible_posts"),
            "no visible posts"
        );
        assert_eq!(
            super::admin_followed_community_health_label("catalog_stale"),
            "catalog stale"
        );
        assert_eq!(super::admin_followed_community_health_label("ok"), "ok");
    }

    #[test]
    fn admin_bytes_label_uses_small_readable_units() {
        assert_eq!(super::admin_bytes_label(-1), "0 B");
        assert_eq!(super::admin_bytes_label(42), "42 B");
        assert_eq!(super::admin_bytes_label(2048), "2.0 KiB");
        assert_eq!(super::admin_bytes_label(5 * 1024 * 1024), "5.0 MiB");
        assert_eq!(super::admin_bytes_label(1536 * 1024 * 1024), "1.5 GiB");
    }

    #[test]
    fn admin_submit_includes_task_and_discovery_settings() {
        assert!(
            super::ADMIN_BOOLEAN_SITE_SETTINGS
                .contains(&"cleanup_failed_inbox_task_payloads_enabled")
        );

        for setting in [
            "cleanup_completed_task_retention_days",
            "cleanup_failed_task_retention_days",
            "cleanup_failed_inbox_task_payload_compaction_hours",
            "discovery_enqueue_limit",
            "discovery_refresh_interval_hours",
        ] {
            assert!(super::ADMIN_NUMERIC_SITE_SETTINGS.contains(&setting));
        }
    }
}
