use crate::components::{
    CommunityLink, ContentView, FollowFederationStatusBadge, HTPage, HTPageAdvanced,
    MaybeFillCheckbox, MaybeFillInput, MaybeFillOption, MaybeFillTextArea, PostItem, SafeTimeAgo,
    federation_status_line_class, maybe_fill_value,
};
use crate::hyper;
use crate::lang;
use crate::query_types::PostListQuery;
use crate::resp_types::{
    JustContentHTML, JustStringID, RespCommunityInfoMaybeYour, RespCommunityModlogEvent,
    RespCommunityModlogEventDetails, RespCommunityVisibilitySuppression, RespList,
    RespMinimalAuthorInfo, RespPostListPost, RespYourFollow,
};
use crate::routes::{
    CookieMap, RespUserInfo, cache_invalidating_response, fetch_base_data, for_client,
    get_cookie_map_for_headers, get_cookie_map_for_req, html_response, res_to_error,
};
use crate::util::safe_href;
use serde_derive::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Deserialize, Debug)]
struct RespCommunitySoftwareCount<'a> {
    software: Cow<'a, str>,
    count: i64,
}

#[derive(Deserialize, Debug)]
struct RespCommunitiesList<'a> {
    #[serde(borrow)]
    items: Vec<RespCommunityInfoMaybeYour<'a>>,
    next_page: Option<Cow<'a, str>>,
    #[serde(default)]
    total_count: Option<i64>,
    #[serde(default)]
    scope_total_count: Option<i64>,
    #[serde(default)]
    software_counts: Vec<RespCommunitySoftwareCount<'a>>,
}

fn communities_url(
    search: Option<&str>,
    scope: &str,
    page: Option<i64>,
    software: Option<&str>,
    sort: Option<&str>,
) -> Result<String, crate::Error> {
    #[derive(Serialize)]
    struct Query<'a> {
        #[serde(skip_serializing_if = "Option::is_none")]
        search: Option<&'a str>,
        scope: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        page: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        software: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        sort: Option<&'a str>,
    }

    let query = serde_urlencoded::to_string(Query {
        search,
        scope,
        page,
        software: software.filter(|software| *software != "all"),
        sort: sort.filter(|sort| *sort != "alphabetic"),
    })?;

    if query.is_empty() {
        Ok("/communities".to_owned())
    } else {
        Ok(format!("/communities?{query}"))
    }
}

enum CommunitiesPaginationItem {
    Page {
        page: i64,
        href: Option<String>,
        current: bool,
    },
    Gap,
}

struct CommunitiesPagination {
    items: Vec<CommunitiesPaginationItem>,
}

impl render::Render for CommunitiesPagination {
    fn render_into<W: std::fmt::Write + ?Sized>(self, writer: &mut W) -> std::fmt::Result {
        if self.items.len() <= 1 {
            return Ok(());
        }

        write!(
            writer,
            "<nav class=\"pagination\" aria-label=\"Community pages\">"
        )?;

        for item in self.items {
            match item {
                CommunitiesPaginationItem::Page {
                    page,
                    href,
                    current,
                } => {
                    if current {
                        write!(writer, "<span aria-current=\"page\">{page}</span>")?;
                    } else if let Some(href) = href {
                        write!(writer, "<a href=\"")?;
                        render::html_escaping::escape_html(&href, writer)?;
                        write!(writer, "\">{page}</a>")?;
                    }
                }
                CommunitiesPaginationItem::Gap => {
                    write!(writer, "<span class=\"paginationGap\">...</span>")?;
                }
            }
        }

        write!(writer, "</nav>")
    }
}

fn communities_page_numbers(current_page: i64, has_next_page: bool) -> Vec<Option<i64>> {
    let current_page = current_page.max(1);
    let mut pages = vec![Some(1)];
    let first_window_page = (current_page - 2).max(2);

    if first_window_page > 2 {
        pages.push(None);
    }

    for page in first_window_page..=current_page {
        if page != 1 {
            pages.push(Some(page));
        }
    }

    if has_next_page {
        pages.push(Some(current_page + 1));
    }

    pages
}

fn render_communities_pagination(
    search: Option<&str>,
    scope: &str,
    current_page: i64,
    has_next_page: bool,
    software: Option<&str>,
    sort: Option<&str>,
) -> Result<CommunitiesPagination, crate::Error> {
    let current_page = current_page.max(1);
    let mut items = Vec::new();

    for page in communities_page_numbers(current_page, has_next_page) {
        match page {
            Some(page) => {
                let href = if page == current_page {
                    None
                } else {
                    let query_page = if page > 1 { Some(page) } else { None };

                    communities_url(search, scope, query_page, software, sort).map(Some)?
                };

                items.push(CommunitiesPaginationItem::Page {
                    page,
                    href,
                    current: page == current_page,
                });
            }
            None => items.push(CommunitiesPaginationItem::Gap),
        }
    }

    Ok(CommunitiesPagination { items })
}

fn normalize_community_software(value: Option<&str>) -> &'static str {
    match value.unwrap_or("all") {
        "local" => "local",
        "lotide" => "lotide",
        "lemmy" => "lemmy",
        "piefed" => "piefed",
        "kbin" => "kbin",
        "mbin" => "mbin",
        "nodebb" => "nodebb",
        "discourse" => "discourse",
        "friendica" => "friendica",
        "mobilizon" => "mobilizon",
        "peertube" => "peertube",
        "smithereen" => "smithereen",
        "hubzilla" => "hubzilla",
        "streams_forte" => "streams_forte",
        "bonfire" => "bonfire",
        "flipboard" => "flipboard",
        "elgg" => "elgg",
        "gancio" => "gancio",
        "funkwhale" => "funkwhale",
        "wordpress" => "wordpress",
        "guppe" => "guppe",
        "fedigroups" => "fedigroups",
        "fedigroup" => "fedigroup",
        "ap_groups" => "ap_groups",
        "group_actor" => "group_actor",
        "tootgroup" => "tootgroup",
        "buzzrelay" => "buzzrelay",
        "mastodon" => "mastodon",
        "pleroma" => "pleroma",
        "unknown" => "unknown",
        _ => "all",
    }
}

fn community_software_label(software: &str) -> Cow<'static, str> {
    match software {
        "all" => Cow::Borrowed("All"),
        "local" => Cow::Borrowed("Local"),
        "lotide" => Cow::Borrowed("Lotide"),
        "lemmy" => Cow::Borrowed("Lemmy"),
        "piefed" => Cow::Borrowed("PieFed"),
        "kbin" => Cow::Borrowed("Kbin"),
        "mbin" => Cow::Borrowed("Mbin"),
        "nodebb" => Cow::Borrowed("NodeBB"),
        "discourse" => Cow::Borrowed("Discourse"),
        "friendica" => Cow::Borrowed("Friendica"),
        "mobilizon" => Cow::Borrowed("Mobilizon"),
        "peertube" => Cow::Borrowed("PeerTube"),
        "smithereen" => Cow::Borrowed("Smithereen"),
        "hubzilla" => Cow::Borrowed("Hubzilla"),
        "streams_forte" => Cow::Borrowed("Streams/Forte"),
        "bonfire" => Cow::Borrowed("Bonfire"),
        "flipboard" => Cow::Borrowed("Flipboard"),
        "elgg" => Cow::Borrowed("Elgg"),
        "gancio" => Cow::Borrowed("Gancio"),
        "funkwhale" => Cow::Borrowed("Funkwhale"),
        "wordpress" => Cow::Borrowed("WordPress"),
        "guppe" => Cow::Borrowed("Guppe"),
        "fedigroups" => Cow::Borrowed("FediGroups"),
        "fedigroup" => Cow::Borrowed("Fedigroup"),
        "ap_groups" => Cow::Borrowed("AP-Groups"),
        "group_actor" => Cow::Borrowed("Group Actor"),
        "tootgroup" => Cow::Borrowed("tootgroup.py"),
        "buzzrelay" => Cow::Borrowed("BuzzRelay"),
        "mastodon" => Cow::Borrowed("Mastodon"),
        "pleroma" => Cow::Borrowed("Pleroma/Akkoma"),
        "unknown" => Cow::Borrowed("Unclassified"),
        other => Cow::Owned(other.replace('_', " ")),
    }
}

fn normalize_community_sort(value: Option<&str>) -> &'static str {
    match value.unwrap_or("alphabetic") {
        "last_post" => "last_post",
        "post_count" => "post_count",
        "host" => "host",
        _ => "alphabetic",
    }
}

fn community_sort_label(sort: &str) -> &'static str {
    match sort {
        "last_post" => "Latest post",
        "post_count" => "Most posts",
        "host" => "Server",
        _ => "Alphabetical",
    }
}

fn communities_count_text(scope: &str, count: i64, search_active: bool) -> String {
    let count = count.max(0);

    match (scope, count, search_active) {
        ("mine", 1, false) => "1 subscribed community".to_owned(),
        ("mine", 1, true) => "1 matching subscribed community".to_owned(),
        ("mine", _, false) => format!("{count} subscribed communities"),
        ("mine", _, true) => format!("{count} matching subscribed communities"),
        (_, 1, false) => "1 community".to_owned(),
        (_, 1, true) => "1 matching community".to_owned(),
        (_, _, false) => format!("{count} communities"),
        (_, _, true) => format!("{count} matching communities"),
    }
}

fn community_activity_text(
    last_post_title: Option<&str>,
    remote_post_count: Option<i64>,
) -> String {
    if let Some(last_post_title) = last_post_title {
        return format!("Last post: {last_post_title}");
    }

    match remote_post_count {
        Some(1) => "Remote reports 1 post".to_owned(),
        Some(count) if count > 1 => format!("Remote reports {count} posts"),
        _ => "No posts seen".to_owned(),
    }
}

fn community_action_return_location(req: &hyper::Request<hyper::Body>, fallback: String) -> String {
    let from_referer = req
        .headers()
        .get(hyper::header::REFERER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| url::Url::parse(value).ok())
        .and_then(|url| {
            let path = url.path();

            if path == "/communities" || path.starts_with("/communities/") {
                let mut result = path.to_owned();

                if let Some(query) = url.query() {
                    result.push('?');
                    result.push_str(query);
                }

                Some(result)
            } else {
                None
            }
        });

    from_referer.unwrap_or(fallback)
}

pub(super) fn community_visibility_suppression_text<'a>(
    lang: &'a crate::Translator,
    suppression: Option<&RespCommunityVisibilitySuppression>,
) -> Option<Cow<'a, str>> {
    let suppression = suppression?;

    if suppression.server {
        Some(lang.tr(&lang::COMMUNITY_VISIBILITY_SERVER_BLOCKED))
    } else if suppression.user {
        Some(lang.tr(&lang::COMMUNITY_VISIBILITY_USER_BLOCKED))
    } else {
        None
    }
}

async fn page_communities(
    (): (),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let lang = crate::get_lang_for_req(&req);
    let cookies = get_cookie_map_for_req(&req)?;
    let base_data =
        fetch_base_data(&ctx.backend_host, &ctx.http_client, req.headers(), &cookies).await?;

    #[derive(Deserialize, Serialize)]
    struct Query<'a> {
        search: Option<Cow<'a, str>>,
        scope: Option<Cow<'a, str>>,
        page: Option<i64>,
        software: Option<Cow<'a, str>>,
        sort: Option<Cow<'a, str>>,
    }

    let query: Query = serde_urlencoded::from_str(req.uri().query().unwrap_or(""))?;
    let default_scope = if base_data.login.is_some() {
        "mine"
    } else {
        "everything"
    };
    let scope = match query.scope.as_deref() {
        Some("mine") if base_data.login.is_some() => "mine",
        Some("everything") => "everything",
        _ => default_scope,
    };
    let your_follow_accepted = if scope == "mine" { Some(true) } else { None };
    let current_page = query.page.filter(|page| *page > 0).unwrap_or(1);
    let software = normalize_community_software(query.software.as_deref());
    let software_for_api = if software == "all" {
        None
    } else {
        Some(software)
    };
    let sort = normalize_community_sort(query.sort.as_deref());

    #[derive(Serialize)]
    struct ApiQuery<'a> {
        #[serde(skip_serializing_if = "Option::is_none")]
        search: Option<&'a str>,
        #[serde(rename = "your_follow.accepted")]
        #[serde(skip_serializing_if = "Option::is_none")]
        your_follow_accepted: Option<bool>,
        scope: &'a str,
        include_your: bool,
        limit: i64,
        #[serde(skip_serializing_if = "Option::is_none")]
        page_number: Option<i64>,
        sort: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        software: Option<&'a str>,
    }

    let api_query = ApiQuery {
        search: query.search.as_deref(),
        your_follow_accepted,
        scope,
        include_your: base_data.login.is_some(),
        limit: 150,
        page_number: Some(current_page),
        sort,
        software: software_for_api,
    };

    let api_res = crate::read_body_with_timeout(
        res_to_error(
            ctx.http_client
                .request(for_client(
                    hyper::Request::get(format!(
                        "{}/api/unstable/communities?{}",
                        ctx.backend_host,
                        serde_urlencoded::to_string(&api_query)?,
                    ))
                    .body(Default::default())?,
                    req.headers(),
                    &cookies,
                )?)
                .await?,
        )
        .await?
        .into_body(),
    )
    .await?;

    let communities: RespCommunitiesList = serde_json::from_slice(&api_res)?;
    let communities_count_text = communities.total_count.map(|count| {
        communities_count_text(
            scope,
            count,
            query
                .search
                .as_deref()
                .is_some_and(|search| !search.trim().is_empty()),
        )
    });

    let title = lang.tr(&lang::COMMUNITIES);

    let filter_options: &[(lang::LangKey, bool, &str)] = &[
        (
            lang::COMMUNITIES_FILTER_MINE,
            base_data.login.is_some(),
            "mine",
        ),
        (lang::COMMUNITIES_FILTER_ALL, true, "everything"),
    ];
    let search_for_links = query
        .search
        .as_deref()
        .filter(|search| !search.trim().is_empty());
    let sort_options = &["alphabetic", "last_post", "post_count", "host"];
    let all_count = communities
        .scope_total_count
        .or(communities.total_count)
        .unwrap_or(0);
    let mut software_options = Vec::with_capacity(communities.software_counts.len() + 1);
    software_options.push(RespCommunitySoftwareCount {
        software: Cow::Borrowed("all"),
        count: all_count,
    });
    software_options.extend(communities.software_counts.iter().map(|software_count| {
        RespCommunitySoftwareCount {
            software: Cow::Borrowed(software_count.software.as_ref()),
            count: software_count.count,
        }
    }));

    Ok(html_response(render::html! {
        <HTPage
            base_data={&base_data}
            lang={&lang}
            title={&title}
        >
            <h1>{title.as_ref()}</h1>
            {
                if let Some(login) = &base_data.login {
                    if login.permissions.create_community.allowed {
                        Some(render::rsx! { <a href={"/new_community"}>{lang.tr(&lang::COMMUNITY_CREATE)}</a> })
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            <form method={"GET"} action={"/lookup"}>
                <label>
                    {lang.tr(&lang::ADD_BY_REMOTE_ID)}{" "}
                    <input r#type={"text"} name={"query"} placeholder={"group@example.com"} />
                </label>
                {" "}
                <button r#type={"submit"}>{lang.tr(&lang::FETCH)}</button>
            </form>
            <div class={"communitiesControls"}>
                <form class={"communitySearch"} method={"GET"} action={"/communities"}>
                    <label>
                        {"Search"}{" "}
                        <input
                            r#type={"text"}
                            name={"search"}
                            value={query.search.as_deref().unwrap_or("")}
                            placeholder={"Search communities"}
                        />
                    </label>
                    <input r#type={"hidden"} name={"scope"} value={scope} />
                    {
                        if software == "all" {
                            None
                        } else {
                            Some(render::rsx! {
                                <input r#type={"hidden"} name={"software"} value={software} />
                            })
                        }
                    }
                    {
                        if sort == "alphabetic" {
                            None
                        } else {
                            Some(render::rsx! {
                                <input r#type={"hidden"} name={"sort"} value={sort} />
                            })
                        }
                    }
                    {" "}
                    <button r#type={"submit"}>{"Search"}</button>
                </form>
                {
                    if base_data.login.is_some() {
                        Some(render::rsx! {
                            <form class={"communitiesBulk"} method={"POST"} action={"/communities/unfollow_inactive"}>
                                <button r#type={"submit"}>{"Unfollow communities with no posts"}</button>
                            </form>
                        })
                    } else {
                        None
                    }
                }
            </div>
            <div class={"sortOptions"}>
                {
                    filter_options.iter()
                        .map(|(key, show, option_scope)| {
                            if *show {
                                let name = lang.tr(key);
                                Ok::<_, crate::Error>(Some(if scope == *option_scope {
                                    render::rsx! { <span>{name}</span> }
                                } else {
                                    let href = communities_url(
                                        search_for_links,
                                        option_scope,
                                        None,
                                        Some(software),
                                        Some(sort),
                                    )?;

                                    render::rsx! { <a href={href}>{name}</a> }
                                }))
                            } else {
                                Ok::<_, crate::Error>(None)
                            }
                        })
                        .collect::<Result<Vec<_>, _>>()?
                }
            </div>
            <div class={"sortOptions communitySoftwareOptions"}>
                {
                    software_options.iter()
                        .filter(|software_count| {
                            software_count.count > 0 || software_count.software.as_ref() == "all"
                        })
                        .map(|software_count| {
                            let option_software = software_count.software.as_ref();
                            let label = format!(
                                "{} ({})",
                                community_software_label(option_software),
                                software_count.count.max(0),
                            );

                            if software == option_software {
                                Ok::<_, crate::Error>(render::rsx! { <span>{label}</span> })
                            } else {
                                let href = communities_url(
                                    search_for_links,
                                    scope,
                                    None,
                                    Some(option_software),
                                    Some(sort),
                                )?;

                                Ok(render::rsx! { <a href={href}>{label}</a> })
                            }
                        })
                        .collect::<Result<Vec<_>, _>>()?
                }
            </div>
            <div class={"sortOptions communitySortOptions"}>
                {
                    sort_options.iter()
                        .map(|option_sort| {
                            let label = community_sort_label(option_sort);

                            if sort == *option_sort {
                                Ok::<_, crate::Error>(render::rsx! { <span>{label}</span> })
                            } else {
                                let href = communities_url(
                                    search_for_links,
                                    scope,
                                    None,
                                    Some(software),
                                    Some(option_sort),
                                )?;

                                Ok(render::rsx! { <a href={href}>{label}</a> })
                            }
                        })
                        .collect::<Result<Vec<_>, _>>()?
                }
            </div>
            {
                communities_count_text.as_deref().map(|count_text| {
                    render::rsx! {
                        <p class={"communitiesCount"}>{count_text}</p>
                    }
                })
            }
            <ul class={"communityList"}>
                {
                    communities.items.iter()
                        .map(|community| {
                            let last_post_text = community_activity_text(
                                community
                                    .last_post
                                    .as_ref()
                                    .map(|last_post| last_post.base.title.as_ref()),
                                community.remote_post_count,
                            );
                            let visibility_notice = community_visibility_suppression_text(
                                &lang,
                                community.visibility_suppression.as_ref(),
                            );

                            render::rsx! {
                                <li>
                                    <div>
                                        <div class={"communityNameLine"}>
                                            <CommunityLink community={community.as_ref()} />
                                        </div>
                                        <small class={"communityMeta"}>
                                            {
                                                if community.base.local {
                                                    Cow::Borrowed("local")
                                                } else {
                                                    Cow::Owned(format!("@{}@{}", community.base.name, community.base.host))
                                                }
                                            }
                                        </small>
                                        <small class={"communityLastPost"}>{last_post_text}</small>
                                        {
                                            visibility_notice.map(|visibility_notice| {
                                                render::rsx! {
                                                    <small class={"communityWarning"}>{visibility_notice}</small>
                                                }
                                            })
                                        }
                                    </div>
                                    {
                            if base_data.login.is_some() {
                                            let action_status = community
                                                .your_follow
                                                .as_ref()
                                                .map_or(community.latest_unfollow_status, |follow| follow.federation_status);
                                            let action_class = format!(
                                                "communityActions {}",
                                                federation_status_line_class(action_status),
                                            );
                                            let (action, label) = match &community.your_follow {
                                                Some(follow) => (
                                                    format!("/communities/{}/unfollow", community.base.id),
                                                    if follow.accepted { "Unfollow" } else { "Cancel follow" },
                                                ),
                                                None => (
                                                    format!("/communities/{}/follow", community.base.id),
                                                    "Follow",
                                                ),
                                            };

                                            Some(render::rsx! {
                                                <div class={action_class}>
                                                    <form method={"POST"} action={action}>
                                                        <button r#type={"submit"}>{label}</button>
                                                    </form>
                                                    <FollowFederationStatusBadge
                                                        your_follow={community.your_follow.as_ref()}
                                                        latest_unfollow_status={community.latest_unfollow_status}
                                                    />
                                                </div>
                                            })
                                        } else {
                                            None
                                        }
                                    }
                                </li>
                            }
                        })
                        .collect::<Vec<_>>()
                }
            </ul>
            {
                render_communities_pagination(
                    search_for_links,
                    scope,
                    current_page,
                    communities.next_page.is_some(),
                    Some(software),
                    Some(sort),
                )?
            }
        </HTPage>
    }))
}

async fn page_community(
    params: (i64,),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (community_id,) = params;

    fn default_sort() -> crate::SortType {
        crate::SortType::Hot
    }

    #[derive(Deserialize)]
    struct Query<'a> {
        #[serde(default = "default_sort")]
        sort: crate::SortType,

        created_within: Option<Cow<'a, str>>,

        page: Option<Cow<'a, str>>,
    }

    let query: Query = serde_urlencoded::from_str(req.uri().query().unwrap_or(""))?;

    let lang = crate::get_lang_for_req(&req);
    let cookies = get_cookie_map_for_req(&req)?;

    // TODO parallelize requests

    let base_data =
        fetch_base_data(&ctx.backend_host, &ctx.http_client, req.headers(), &cookies).await?;

    let community_info_api_res = res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::get(format!(
                    "{}/api/unstable/communities/{}{}",
                    ctx.backend_host,
                    community_id,
                    if base_data.login.is_some() {
                        "?include_your=true"
                    } else {
                        ""
                    },
                ))
                .body(Default::default())?,
                req.headers(),
                &cookies,
            )?)
            .await?,
    )
    .await?;
    let community_info_api_res =
        crate::read_body_with_timeout(community_info_api_res.into_body()).await?;

    let community_info: RespCommunityInfoMaybeYour =
        { serde_json::from_slice(&community_info_api_res)? };

    let posts_api_res = res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::get(format!(
                    "{}/api/unstable/posts?{}",
                    ctx.backend_host,
                    serde_urlencoded::to_string(&PostListQuery {
                        community: Some(community_id),
                        created_within: query.created_within.as_deref(),
                        sort_sticky: Some(query.sort == crate::SortType::Hot),
                        sort: Some(query.sort.as_str()),
                        page: query.page.as_deref(),
                        ..Default::default()
                    })?,
                ))
                .body(Default::default())?,
                req.headers(),
                &cookies,
            )?)
            .await?,
    )
    .await?;
    let posts_api_res = crate::read_body_with_timeout(posts_api_res.into_body()).await?;

    let posts: RespList<RespPostListPost<'_>> = serde_json::from_slice(&posts_api_res)?;

    let new_post_url = format!("/communities/{community_id}/new_post");

    let title = community_info.as_ref().name.as_ref();

    let feed_url = &community_info.feeds.atom.new;
    let visibility_notice = community_visibility_suppression_text(
        &lang,
        community_info.visibility_suppression.as_ref(),
    );
    let interaction_blocked = community_info
        .visibility_suppression
        .as_ref()
        .is_some_and(|suppression| suppression.server || suppression.user);
    let follow_status = community_info
        .your_follow
        .as_ref()
        .map_or(community_info.latest_unfollow_status, |follow| {
            follow.federation_status
        });
    let follow_class = format!(
        "followAction {}",
        federation_status_line_class(follow_status),
    );

    let basic_info_area = render::rsx! {
        <div class={"communityBaseInfo"}>
            <h2><a href={format!("/communities/{}", community_id)}>{title}</a></h2>
            <div><em>{format!("@{}@{}", community_info.as_ref().name, community_info.as_ref().host)}</em></div>
            {
                if community_info.as_ref().local {
                    None
                } else {
                    community_info.as_ref().remote_url.as_ref().map(|remote_url| render::rsx! {
                        <div class={"infoBox"}>
                            {lang.tr(&lang::COMMUNITY_REMOTE_NOTE)}
                            {" "}
                            <a href={safe_href(remote_url).unwrap_or("#")}>{lang.tr(&lang::VIEW_AT_SOURCE)}{" ↗"}</a>
                        </div>
                    })
                }
            }
            {
                visibility_notice.map(|visibility_notice| {
                    render::rsx! {
                        <div class={"infoBox"}>
                            {visibility_notice}
                        </div>
                    }
                })
            }
            <p>
                {
                    if interaction_blocked {
                        None
                    } else {
                        Some(render::rsx! { <a href={&new_post_url}>{lang.tr(&lang::POST_NEW)}</a> })
                    }
                }
            </p>
        </div>
    };

    let details_content = render::rsx! {
        <>
            {
                if base_data.login.is_some() {
                    Some(render::rsx! {
                        <p class={follow_class}>
                            {
                                match community_info.your_follow.as_ref() {
                                    Some(RespYourFollow { accepted: true, .. }) => {
                                        render::rsx! {
                                            <form method={"POST"} action={format!("/communities/{}/unfollow", community_id)}>
                                                <button type={"submit"}>{lang.tr(&lang::FOLLOW_UNDO)}</button>
                                            </form>
                                        }
                                    },
                                    Some(RespYourFollow { accepted: false, .. }) => {
                                        render::rsx! {
                                            <form method={"POST"} action={format!("/communities/{}/unfollow", community_id)}>
                                                <button type={"submit"}>{lang.tr(&lang::FOLLOW_REQUEST_CANCEL)}</button>
                                            </form>
                                        }
                                    },
                                    None => {
                                        render::rsx! {
                                            <form method={"POST"} action={format!("/communities/{}/follow", community_id)}>
                                                <button type={"submit"}>{lang.tr(&lang::FOLLOW)}</button>
                                            </form>
                                        }
                                    }
                                }
                            }
                            <FollowFederationStatusBadge
                                your_follow={community_info.your_follow.as_ref()}
                                latest_unfollow_status={community_info.latest_unfollow_status}
                            />
                        </p>
                    })
                } else {
                    None
                }
            }
            {
                if community_info.you_are_moderator == Some(true) {
                    Some(render::rsx! {
                        <>
                            <p>
                                <a href={format!("/communities/{}/edit", community_id)}>{lang.tr(&lang::COMMUNITY_EDIT_LINK)}</a>
                            </p>
                            <p>
                                <a href={format!("/flags?to_community={}", community_id)}>{lang.tr(&lang::COMMUNITY_FLAGS_LINK)}</a>
                            </p>
                        </>
                    })
                } else {
                    None
                }
            }
            <ContentView src={&community_info.description} />
            {
                if community_info.as_ref().local {
                    Some(render::rsx! {
                        <>
                            <p>
                                <a href={format!("/communities/{}/moderators", community_id)}>
                                    {lang.tr(&lang::MODERATORS)}
                                </a>
                            </p>
                            <p>
                                <a href={format!("/communities/{}/modlog", community_id)}>
                                    {lang.tr(&lang::MODLOG)}
                                </a>
                            </p>
                        </>
                    })
                } else {
                    None
                }
            }
            {
                if community_info.you_are_moderator == Some(true) || base_data.is_site_admin() {
                    Some(render::rsx! {
                        <p>
                            <a href={format!("/communities/{}/delete", community_id)}>{lang.tr(&lang::COMMUNITY_DELETE_LINK)}</a>
                        </p>
                    })
                } else {
                    None
                }
            }
        </>
    };

    Ok(html_response(render::html! {
        <HTPageAdvanced
            base_data={&base_data}
            lang={&lang}
            title
            head_items={render::rsx! {
                <link rel={"alternate"} type={"application/atom+xml"} href={safe_href(feed_url).unwrap_or("#")} />
            }}
        >
            <div class={"communityDetailsMobile"}>
                {basic_info_area.clone()}
                <details>
                    {details_content.clone()}
                </details>
                <hr />
            </div>
            <div class={"communitySidebar"}>
                {basic_info_area}
                {details_content}
            </div>
            <div class={"sortOptions"}>
                <span>{lang.tr(&lang::sort())}</span>
                {
                    crate::SortType::VALUES.iter()
                        .map(|value| {
                            let name = lang.tr(&value.lang_key()).into_owned();
                            if query.sort == *value {
                                render::rsx! { <span>{name}</span> }
                            } else {
                                render::rsx! { <a href={format!("/communities/{}?sort={}", community_id, value.as_str())}>{name}</a> }
                            }
                        })
                        .collect::<Vec<_>>()
                }
                {
                    (query.sort == crate::SortType::Top)
                        .then(|| {
                            render::rsx! {
                                <div class={"timeframeOptions"}>
                                    <span>{lang.tr(&lang::POST_TIMEFRAME)}</span>
                                    {
                                        [
                                            (lang::TIMEFRAME_ALL, None),
                                            (lang::TIMEFRAME_YEAR, Some("P1Y")),
                                            (lang::TIMEFRAME_MONTH, Some("P1M")),
                                            (lang::TIMEFRAME_WEEK, Some("P1W")),
                                            (lang::TIMEFRAME_DAY, Some("P1D")),
                                            (lang::TIMEFRAME_HOUR, Some("PT1H")),
                                        ]
                                            .iter()
                                            .map(|(key, interval)| {
                                                let name = lang.tr(key);
                                                if query.created_within.as_deref() == *interval {
                                                    render::rsx! { <span>{name}</span> }
                                                } else if let Some(interval) = interval {
                                                    render::rsx! { <a href={format!("/communities/{}?sort=top&created_within={}", community_id, interval)}>{name}</a> }
                                                } else {
                                                    render::rsx! { <a href={format!("/communities/{}?sort=top", community_id)}>{name}</a> }
                                                }
                                            })
                                            .collect::<Vec<_>>()
                                    }
                                </div>
                            }
                        })
                }
            </div>
            {
                if posts.items.is_empty() {
                    Some(render::rsx! { <p>{lang.tr(&lang::NOTHING)}</p> })
                } else {
                    None
                }
            }
            <ul>
                {posts.items.iter().map(|post| {
                    PostItem { post, in_community: true, no_user: false, lang: &lang }
                }).collect::<Vec<_>>()}
            </ul>
            {
                if let Some(next_page) = &posts.next_page {
                    Some(render::rsx! {
                        <a href={format!("/communities/{}?sort={}&page={}", community_id, query.sort.as_str(), next_page)}>
                            {lang.tr(&lang::POSTS_PAGE_NEXT)}
                        </a>
                    })
                } else {
                    None
                }
            }
        </HTPageAdvanced>
    }))
}

async fn page_community_edit(
    params: (i64,),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (community_id,) = params;

    let cookies = get_cookie_map_for_req(&req)?;

    page_community_edit_inner(community_id, req.headers(), &cookies, ctx, None, None).await
}

async fn page_community_edit_inner(
    community_id: i64,
    headers: &hyper::header::HeaderMap,
    cookies: &CookieMap<'_>,
    ctx: Arc<crate::RouteContext>,
    display_error: Option<String>,
    prev_values: Option<&HashMap<&str, serde_json::Value>>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let base_data = fetch_base_data(&ctx.backend_host, &ctx.http_client, headers, cookies).await?;
    let lang = crate::get_lang_for_headers(headers);

    let community_info_api_res = res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::get(format!(
                    "{}/api/unstable/communities/{}",
                    ctx.backend_host, community_id,
                ))
                .body(Default::default())?,
                headers,
                cookies,
            )?)
            .await?,
    )
    .await?;
    let community_info_api_res =
        crate::read_body_with_timeout(community_info_api_res.into_body()).await?;

    let community_info: RespCommunityInfoMaybeYour =
        { serde_json::from_slice(&community_info_api_res)? };

    let title = lang.tr(&lang::COMMUNITY_EDIT);

    Ok(html_response(render::html! {
        <HTPage base_data={&base_data} lang={&lang} title={&title}>
            <h1>{title.as_ref()}</h1>
            <h2>{community_info.as_ref().name.as_ref()}</h2>
            {
                display_error.map(|msg| {
                    render::rsx! {
                        <div class={"errorBox"}>{msg}</div>
                    }
                })
            }
            <form method={"POST"} action={format!("/communities/{}/edit/submit", community_id)}>
                <label>
                    {lang.tr(&lang::description())}{":"}<br />
                    <MaybeFillTextArea values={&prev_values} name={"description_markdown"} default_value={Some(community_info.description.content_markdown.as_deref().or(community_info.description.content_html.as_deref()).or(community_info.description.content_text.as_deref()).unwrap())} />
                </label>
                <div>
                    <button r#type={"submit"}>{lang.tr(&lang::submit())}</button>
                </div>
            </form>
        </HTPage>
    }))
}

async fn handler_communities_edit_submit(
    params: (i64,),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (community_id,) = params;

    let (req_parts, body) = req.into_parts();

    let cookies = get_cookie_map_for_headers(&req_parts.headers)?;

    let body = crate::read_body_with_timeout(body).await?;
    let body: HashMap<&str, serde_json::Value> = serde_urlencoded::from_bytes(&body)?;

    let api_res = res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::patch(format!(
                    "{}/api/unstable/communities/{}",
                    ctx.backend_host, community_id
                ))
                .body(serde_json::to_vec(&body)?.into())?,
                &req_parts.headers,
                &cookies,
            )?)
            .await?,
    )
    .await;

    match api_res {
        Err(crate::Error::RemoteError((_, message))) => {
            page_community_edit_inner(
                community_id,
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
            .header(
                hyper::header::LOCATION,
                format!("/communities/{community_id}"),
            )
            .body("Successfully edited.".into())?),
    }
}

async fn page_community_delete(
    params: (i64,),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (community_id,) = params;

    let lang = crate::get_lang_for_req(&req);
    let cookies = get_cookie_map_for_req(&req)?;

    let base_data =
        fetch_base_data(&ctx.backend_host, &ctx.http_client, req.headers(), &cookies).await?;

    let api_res = res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::get(format!(
                    "{}/api/unstable/communities/{}",
                    ctx.backend_host, community_id
                ))
                .body(Default::default())?,
                req.headers(),
                &cookies,
            )?)
            .await?,
    )
    .await?;
    let api_res = crate::read_body_with_timeout(api_res.into_body()).await?;

    let community: RespCommunityInfoMaybeYour = serde_json::from_slice(&api_res)?;

    Ok(html_response(render::html! {
        <HTPage base_data={&base_data} lang={&lang} title={&lang.tr(&lang::community_delete_title())}>
            <h1>{community.as_ref().name.as_ref()}</h1>
            <h2>{lang.tr(&lang::community_delete_question())}</h2>
            <form method={"POST"} action={format!("/communities/{}/delete/confirm", community.as_ref().id)}>
                <a href={format!("/communities/{}/", community.as_ref().id)}>{lang.tr(&lang::no_cancel())}</a>
                {" "}
                <button r#type={"submit"}>{lang.tr(&lang::delete_yes())}</button>
            </form>
        </HTPage>
    }))
}

async fn handler_community_delete_confirm(
    params: (i64,),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (community_id,) = params;

    let cookies = get_cookie_map_for_req(&req)?;

    res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::delete(format!(
                    "{}/api/unstable/communities/{}",
                    ctx.backend_host, community_id,
                ))
                .body("".into())?,
                req.headers(),
                &cookies,
            )?)
            .await?,
    )
    .await?;

    Ok(hyper::Response::builder()
        .status(hyper::StatusCode::SEE_OTHER)
        .header(hyper::header::LOCATION, "/")
        .body("Successfully deleted.".into())?)
}

async fn handler_community_follow(
    params: (i64,),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (community_id,) = params;

    let cookies = get_cookie_map_for_req(&req)?;
    let location = community_action_return_location(&req, format!("/communities/{community_id}"));

    res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::post(format!(
                    "{}/api/unstable/communities/{}/follow",
                    ctx.backend_host, community_id
                ))
                .header(hyper::header::CONTENT_TYPE, "application/json")
                .body("{\"try_wait_for_accept\":false}".into())?,
                req.headers(),
                &cookies,
            )?)
            .await?,
    )
    .await?;

    Ok(cache_invalidating_response(
        hyper::Response::builder()
            .status(hyper::StatusCode::SEE_OTHER)
            .header(hyper::header::LOCATION, location)
            .body("Successfully followed".into())?,
    ))
}

async fn handler_communities_unfollow_inactive(
    (): (),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let cookies = get_cookie_map_for_req(&req)?;

    res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::post(format!(
                    "{}/api/unstable/communities/unfollow_inactive",
                    ctx.backend_host
                ))
                .body(Default::default())?,
                req.headers(),
                &cookies,
            )?)
            .await?,
    )
    .await?;

    Ok(cache_invalidating_response(
        hyper::Response::builder()
            .status(hyper::StatusCode::SEE_OTHER)
            .header(hyper::header::LOCATION, "/communities?scope=mine")
            .body("Successfully unfollowed inactive communities".into())?,
    ))
}

async fn page_community_moderators(
    params: (i64,),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (community_id,) = params;

    let headers = req.headers();
    let cookies = get_cookie_map_for_req(&req)?;

    page_community_moderators_inner(community_id, headers, &cookies, ctx, None, None).await
}

async fn page_community_moderators_inner(
    community_id: i64,
    headers: &hyper::header::HeaderMap,
    cookies: &CookieMap<'_>,
    ctx: Arc<crate::RouteContext>,
    display_error_main: Option<String>,
    display_error_add: Option<String>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let lang = crate::get_lang_for_headers(headers);

    let base_data = fetch_base_data(&ctx.backend_host, &ctx.http_client, headers, cookies).await?;

    let community_info_api_res = res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::get(format!(
                    "{}/api/unstable/communities/{}{}",
                    ctx.backend_host,
                    community_id,
                    if base_data.login.is_some() {
                        "?include_your=true"
                    } else {
                        ""
                    },
                ))
                .body(Default::default())?,
                headers,
                cookies,
            )?)
            .await?,
    )
    .await?;
    let community_info_api_res =
        crate::read_body_with_timeout(community_info_api_res.into_body()).await?;
    let community_info: RespCommunityInfoMaybeYour =
        { serde_json::from_slice(&community_info_api_res)? };

    let api_res = res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::get(format!(
                    "{}/api/unstable/communities/{}/moderators",
                    ctx.backend_host, community_id,
                ))
                .body(Default::default())?,
                headers,
                cookies,
            )?)
            .await?,
    )
    .await?;
    let api_res = crate::read_body_with_timeout(api_res.into_body()).await?;
    let api_res: Vec<RespMinimalAuthorInfo> = serde_json::from_slice(&api_res)?;

    let title = lang.tr(&lang::MODERATORS);

    Ok(html_response(render::html! {
        <HTPage base_data={&base_data} lang={&lang} title={&title}>
            <h1>{title.as_ref()}</h1>
            {
                display_error_main.map(|msg| {
                    render::rsx! {
                        <div class={"errorBox"}>{msg}</div>
                    }
                })
            }
            <ul>
                {
                    api_res.iter().map(|user| {
                        render::rsx! {
                            <li>
                                <a href={format!("/users/{}", user.id)}>{user.username.as_ref()}</a>
                                {
                                    if community_info.you_are_moderator == Some(true) {
                                        Some(render::rsx! {
                                            <>
                                                {" "}
                                                <form class={"inline"} method={"POST"} action={format!("/communities/{}/moderators/remove", community_id)}>
                                                    <input type={"hidden"} name={"user"} value={user.id.to_string()} />
                                                    <button type={"submit"}>{lang.tr(&lang::REMOVE)}</button>
                                                </form>
                                            </>
                                        })
                                    } else {
                                        None
                                    }
                                }
                            </li>
                        }
                    })
                    .collect::<Vec<_>>()
                }
            </ul>
            {
                if community_info.you_are_moderator == Some(true) {
                    Some(render::rsx! {
                        <div>
                            <h2>{lang.tr(&lang::COMMUNITY_ADD_MODERATOR)}</h2>
                            {
                                display_error_add.map(|msg| {
                                    render::rsx! {
                                        <div class={"errorBox"}>{msg}</div>
                                    }
                                })
                            }
                            <form method={"POST"} action={format!("/communities/{}/moderators/add", community_id)}>
                                <label>
                                    {lang.tr(&lang::LOCAL_USER_NAME_PROMPT)}{" "}
                                    <input type={"text"} name={"username"} />
                                </label>
                                {" "}
                                <button type={"submit"}>{lang.tr(&lang::ADD)}</button>
                            </form>
                        </div>
                    })
                } else {
                    None
                }
            }
        </HTPage>
    }))
}

async fn handler_community_moderators_add(
    params: (i64,),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (community_id,) = params;

    let (req_parts, body) = req.into_parts();

    let lang = crate::get_lang_for_headers(&req_parts.headers);
    let cookies = get_cookie_map_for_headers(&req_parts.headers)?;

    #[derive(Deserialize)]
    struct ModeratorsAddParams<'a> {
        username: Cow<'a, str>,
    }

    let body = crate::read_body_with_timeout(body).await?;
    let body: ModeratorsAddParams = serde_urlencoded::from_bytes(&body)?;

    #[derive(Serialize)]
    struct UsersListQuery<'a> {
        local: bool,
        username: &'a str,
    }

    let user_lookup_api_res = res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::get(format!(
                    "{}/api/unstable/users?{}",
                    ctx.backend_host,
                    serde_urlencoded::to_string(&UsersListQuery {
                        local: true,
                        username: &body.username,
                    })?,
                ))
                .body(Default::default())?,
                &req_parts.headers,
                &cookies,
            )?)
            .await?,
    )
    .await;

    let add_result = match user_lookup_api_res {
        Err(err) => Err(err),
        Ok(api_res) => {
            let value = crate::read_body_with_timeout(api_res.into_body()).await?;
            let user_list: RespList<RespUserInfo> = serde_json::from_slice(&value)?;

            match user_list.items.first() {
                None => Err(crate::Error::InternalUserError(
                    lang.tr(&lang::no_such_local_user()).into_owned(),
                )),
                Some(target_user) => {
                    res_to_error(
                        ctx.http_client
                            .request(for_client(
                                hyper::Request::put(format!(
                                    "{}/api/unstable/communities/{}/moderators/{}",
                                    ctx.backend_host, community_id, target_user.base.id,
                                ))
                                .body(Default::default())?,
                                &req_parts.headers,
                                &cookies,
                            )?)
                            .await?,
                    )
                    .await
                }
            }
        }
    };

    match add_result {
        Err(crate::Error::RemoteError((_, message)) | crate::Error::InternalUserError(message)) => {
            page_community_moderators_inner(
                community_id,
                &req_parts.headers,
                &cookies,
                ctx,
                None,
                Some(message),
            )
            .await
        }
        Err(other) => Err(other),
        Ok(_) => Ok(hyper::Response::builder()
            .status(hyper::StatusCode::SEE_OTHER)
            .header(
                hyper::header::LOCATION,
                format!("/communities/{community_id}/moderators"),
            )
            .body("Successfully added.".into())?),
    }
}

async fn handler_community_moderators_remove(
    params: (i64,),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (community_id,) = params;

    let (req_parts, body) = req.into_parts();

    let cookies = get_cookie_map_for_headers(&req_parts.headers)?;

    #[derive(Deserialize)]
    struct ModeratorsRemoveParams {
        user: i64,
    }

    let body = crate::read_body_with_timeout(body).await?;
    let body: ModeratorsRemoveParams = serde_urlencoded::from_bytes(&body)?;

    let api_res = res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::delete(format!(
                    "{}/api/unstable/communities/{}/moderators/{}",
                    ctx.backend_host, community_id, body.user,
                ))
                .body(Default::default())?,
                &req_parts.headers,
                &cookies,
            )?)
            .await?,
    )
    .await;

    match api_res {
        Err(crate::Error::RemoteError((_, message))) => {
            page_community_moderators_inner(
                community_id,
                &req_parts.headers,
                &cookies,
                ctx,
                Some(message),
                None,
            )
            .await
        }
        Err(other) => Err(other),
        Ok(_) => Ok(hyper::Response::builder()
            .status(hyper::StatusCode::SEE_OTHER)
            .header(
                hyper::header::LOCATION,
                format!("/communities/{community_id}/moderators"),
            )
            .body("Successfully removed.".into())?),
    }
}

async fn page_community_modlog(
    params: (i64,),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (community_id,) = params;

    let lang = crate::get_lang_for_req(&req);
    let cookies = get_cookie_map_for_req(&req)?;

    let base_data =
        fetch_base_data(&ctx.backend_host, &ctx.http_client, req.headers(), &cookies).await?;

    let api_res = res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::get(format!(
                    "{}/api/unstable/communities/{}/modlog/events",
                    ctx.backend_host, community_id,
                ))
                .body(Default::default())?,
                req.headers(),
                &cookies,
            )?)
            .await?,
    )
    .await?;
    let api_res = crate::read_body_with_timeout(api_res.into_body()).await?;
    let api_res: RespList<RespCommunityModlogEvent> = serde_json::from_slice(&api_res)?;

    let title = lang.tr(&lang::MODLOG);

    Ok(html_response(render::html! {
        <HTPage base_data={&base_data} lang={&lang} title={&title}>
            <h1>{title.as_ref()}</h1>
            <ul>
                {
                    api_res.items.iter().map(|event| {
                        render::rsx! {
                            <li>
                                <SafeTimeAgo since={event.time.as_ref()} lang={&lang} />
                                {" - "}
                                {
                                    match &event.details {
                                        RespCommunityModlogEventDetails::ApprovePost { post } => {
                                            render::rsx! {
                                                <>
                                                    {lang.tr(&lang::MODLOG_EVENT_APPROVE_POST)}
                                                    {" "}
                                                    <a href={format!("/posts/{}", post.id)}>{post.title.as_ref()}</a>
                                                </>
                                            }
                                        }
                                        RespCommunityModlogEventDetails::RejectPost { post } => {
                                            render::rsx! {
                                                <>
                                                    {lang.tr(&lang::MODLOG_EVENT_REJECT_POST)}
                                                    {" "}
                                                    <a href={format!("/posts/{}", post.id)}>{post.title.as_ref()}</a>
                                                </>
                                            }
                                        }
                                    }
                                }
                            </li>
                        }
                    })
                    .collect::<Vec<_>>()
                }
            </ul>
        </HTPage>
    }))
}

async fn handler_community_post_approve(
    params: (i64, i64),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (community_id, post_id) = params;

    let cookies = get_cookie_map_for_req(&req)?;

    res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::patch(format!(
                    "{}/api/unstable/communities/{}/posts/{}",
                    ctx.backend_host, community_id, post_id
                ))
                .body("{\"approved\": true}".into())?,
                req.headers(),
                &cookies,
            )?)
            .await?,
    )
    .await?;

    Ok(hyper::Response::builder()
        .status(hyper::StatusCode::SEE_OTHER)
        .header(hyper::header::LOCATION, format!("/posts/{post_id}"))
        .body("Successfully approved.".into())?)
}

async fn handler_community_post_unapprove(
    params: (i64, i64),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (community_id, post_id) = params;

    let cookies = get_cookie_map_for_req(&req)?;

    res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::patch(format!(
                    "{}/api/unstable/communities/{}/posts/{}",
                    ctx.backend_host, community_id, post_id
                ))
                .body("{\"approved\": false}".into())?,
                req.headers(),
                &cookies,
            )?)
            .await?,
    )
    .await?;

    Ok(hyper::Response::builder()
        .status(hyper::StatusCode::SEE_OTHER)
        .header(hyper::header::LOCATION, format!("/posts/{post_id}"))
        .body("Successfully unapproved.".into())?)
}

async fn handler_community_post_make_sticky(
    params: (i64, i64),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (community_id, post_id) = params;

    let cookies = get_cookie_map_for_req(&req)?;

    res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::patch(format!(
                    "{}/api/unstable/communities/{}/posts/{}",
                    ctx.backend_host, community_id, post_id
                ))
                .body("{\"sticky\": true}".into())?,
                req.headers(),
                &cookies,
            )?)
            .await?,
    )
    .await?;

    Ok(hyper::Response::builder()
        .status(hyper::StatusCode::SEE_OTHER)
        .header(hyper::header::LOCATION, format!("/posts/{post_id}"))
        .body("Successfully stickied.".into())?)
}

async fn handler_community_post_make_unsticky(
    params: (i64, i64),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (community_id, post_id) = params;

    let cookies = get_cookie_map_for_req(&req)?;

    res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::patch(format!(
                    "{}/api/unstable/communities/{}/posts/{}",
                    ctx.backend_host, community_id, post_id
                ))
                .body("{\"sticky\": false}".into())?,
                req.headers(),
                &cookies,
            )?)
            .await?,
    )
    .await?;

    Ok(hyper::Response::builder()
        .status(hyper::StatusCode::SEE_OTHER)
        .header(hyper::header::LOCATION, format!("/posts/{post_id}"))
        .body("Successfully unstickied.".into())?)
}

async fn handler_community_unfollow(
    params: (i64,),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (community_id,) = params;

    let cookies = get_cookie_map_for_req(&req)?;
    let location = community_action_return_location(&req, format!("/communities/{community_id}"));

    res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::post(format!(
                    "{}/api/unstable/communities/{}/unfollow",
                    ctx.backend_host, community_id
                ))
                .body(Default::default())?,
                req.headers(),
                &cookies,
            )?)
            .await?,
    )
    .await?;

    Ok(cache_invalidating_response(
        hyper::Response::builder()
            .status(hyper::StatusCode::SEE_OTHER)
            .header(hyper::header::LOCATION, location)
            .body("Successfully unfollowed".into())?,
    ))
}

async fn page_community_new_post(
    params: (i64,),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (community_id,) = params;

    let cookies = get_cookie_map_for_req(&req)?;

    page_community_new_post_inner(community_id, req.headers(), &cookies, ctx, None, None, None)
        .await
}

async fn page_community_new_post_inner(
    community_id: i64,
    headers: &hyper::header::HeaderMap,
    cookies: &CookieMap<'_>,
    ctx: Arc<crate::RouteContext>,
    display_error: Option<String>,
    prev_values: Option<&HashMap<Cow<'_, str>, serde_json::Value>>,
    display_preview: Option<&str>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let base_data = fetch_base_data(&ctx.backend_host, &ctx.http_client, headers, cookies).await?;
    let lang = crate::get_lang_for_headers(headers);

    let submit_url = format!("/communities/{community_id}/new_post/submit");

    let title_key = lang::post_new();
    let title = lang.tr(&title_key);

    let poll_option_names: Vec<_> = (0..4).map(|idx| format!("poll_option_{idx}")).collect();

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
            <form method={"POST"} action={&submit_url} enctype={"multipart/form-data"}>
                <table>
                    <tr>
                        <td>
                            <label for={"input_title"}>{lang.tr(&lang::title())}{":"}</label>
                        </td>
                        <td>
                            <MaybeFillInput values={&prev_values} r#type={"text"} name={"title"} required={true} id={"input_title"} />
                        </td>
                    </tr>
                    <tr>
                        <td>
                            <label for={"input_url"}>{lang.tr(&lang::url())}{":"}</label>
                        </td>
                        <td>
                            <MaybeFillInput values={&prev_values} r#type={"text"} name={"href"} required={false} id={"input_url"} />
                        </td>
                    </tr>
                    <tr>
                        <td>
                            <label for={"input_image"}>{lang.tr(&lang::post_new_image_prompt())}</label>
                        </td>
                        <td>
                            <input id={"input_image"} type={"file"} accept={"image/*"} name={"href_media"} />
                        </td>
                    </tr>
                </table>
                <label>
                    {lang.tr(&lang::text_with_markdown())}{":"}
                    <br />
                    <MaybeFillTextArea values={&prev_values} name={"content_markdown"} default_value={None} />
                </label>
                <br />
                <label>
                    <MaybeFillCheckbox values={&prev_values} id={"sensitiveCheckbox"} name={"sensitive"} default={false} />{" "}
                    {lang.tr(&lang::sensitive()).into_owned()}
                </label>
                <br />
                <MaybeFillCheckbox values={&prev_values} id={"pollEnableCheckbox"} name={"poll_enabled"} default={false} />
                <label for={"pollEnableCheckbox"}>
                    {" "}
                    {lang.tr(&lang::new_post_poll())}
                </label>
                <br />
                <div class={"pollArea"}>
                    <div>
                        <label>
                            <MaybeFillCheckbox values={&prev_values} name={"poll_multiple"} id={"poll_multiple"} default={false} />
                            {" "}
                            {lang.tr(&lang::poll_new_multiple())}
                        </label>
                    </div>
                    {lang.tr(&lang::poll_new_options_prompt())}
                    <ul>
                        {
                            poll_option_names.iter().map(|name| {
                                render::rsx! {
                                    <li><MaybeFillInput values={&prev_values} r#type={"text"} name={name} id={name} required={false} /></li>
                                }
                            })
                            .collect::<Vec<_>>()
                        }
                    </ul>
                    <div>
                        {lang.tr(&lang::poll_new_closes_prompt())}
                        {" "}
                        <input type={"number"} name={"poll_duration_value"} required={""} value={maybe_fill_value(&prev_values, "poll_duration_value", Some("10"))} />
                        <select name={"poll_duration_unit"}>
                            <MaybeFillOption default_value={None} values={&prev_values} name={"poll_duration_unit"} value={"m"}>{lang.tr(&lang::time_input_minutes())}</MaybeFillOption>
                            <MaybeFillOption default_value={None} values={&prev_values} name={"poll_duration_unit"} value={"h"}>{lang.tr(&lang::time_input_hours())}</MaybeFillOption>
                            <MaybeFillOption default_value={None} values={&prev_values} name={"poll_duration_unit"} value={"d"}>{lang.tr(&lang::time_input_days())}</MaybeFillOption>
                        </select>
                    </div>
                </div>
                <div>
                    <button r#type={"submit"}>{lang.tr(&lang::submit())}</button>
                    <button r#type={"submit"} name={"preview"}>{lang.tr(&lang::preview())}</button>
                </div>
            </form>
            {
                display_preview.map(|html| {
                    render::rsx! {
                        <div class={"preview"}>{render::raw!(html)}</div>
                    }
                })
            }
        </HTPage>
    }))
}

async fn handler_communities_new_post_submit(
    params: (i64,),
    ctx: Arc<crate::RouteContext>,
    req: hyper::Request<hyper::Body>,
) -> Result<hyper::Response<hyper::Body>, crate::Error> {
    let (community_id,) = params;

    let (req_parts, body) = req.into_parts();
    let lang = crate::get_lang_for_headers(&req_parts.headers);
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

    let mut body_values_src: HashMap<Cow<'_, str>, serde_json::Value> = HashMap::new();
    {
        let mut error = None;

        loop {
            let field = multipart.next_field().await?;
            let field = match field {
                None => break,
                Some(field) => field,
            };

            if field.name().is_none() {
                continue;
            }

            if field.name().unwrap() == "href_media" {
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

                if body_values_src.contains_key("href") && body_values_src["href"] != "" {
                    error = Some(lang.tr(&lang::post_new_href_conflict()).into_owned());
                } else {
                    match stream.get_ref().content_type() {
                        None => {
                            error =
                                Some(lang.tr(&lang::post_new_missing_content_type()).into_owned());
                        }
                        Some(mime) => {
                            log::debug!("will upload media");
                            let res = res_to_error(
                                ctx.http_client
                                    .request(for_client(
                                        hyper::Request::post(format!(
                                            "{}/api/unstable/media",
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
                                    error = Some(message);
                                }
                                Err(other) => {
                                    return Err(other);
                                }
                                Ok(res) => {
                                    let res =
                                        crate::read_body_with_timeout(res.into_body()).await?;
                                    let res: JustStringID = serde_json::from_slice(&res)?;

                                    body_values_src.insert(
                                        "href".into(),
                                        format!("local-media://{}", res.id).into(),
                                    );
                                }
                            }

                            log::debug!("finished media upload");
                        }
                    }
                }
            } else {
                let name = field.name().unwrap();
                if name == "href"
                    && body_values_src.contains_key("href")
                    && body_values_src["href"] != ""
                {
                    error = Some(lang.tr(&lang::post_new_href_conflict()).into_owned());
                } else {
                    let name = name.to_owned();
                    let value = field.text().await?;
                    body_values_src.insert(name.into(), value.into());
                }
            }
        }

        if let Some(error) = error {
            return page_community_new_post_inner(
                community_id,
                &req_parts.headers,
                &cookies,
                ctx,
                Some(error),
                Some(&body_values_src),
                None,
            )
            .await;
        }
    }

    let body_values_src = body_values_src;
    let mut body_values: HashMap<_, _> = body_values_src
        .iter()
        .map(|(key, value)| (Cow::Borrowed(key.as_ref()), Cow::Borrowed(value)))
        .collect();

    if body_values.contains_key("preview") {
        let md = body_values
            .get("content_markdown")
            .and_then(|x| x.as_str())
            .unwrap_or("");
        let preview_res = res_to_error(
            ctx.http_client
                .request(for_client(
                    hyper::Request::post(format!(
                        "{}/api/unstable/misc/render_markdown",
                        ctx.backend_host
                    ))
                    .body(
                        serde_json::to_vec(&serde_json::json!({ "content_markdown": md }))?.into(),
                    )?,
                    &req_parts.headers,
                    &cookies,
                )?)
                .await?,
        )
        .await;
        return match preview_res {
            Ok(preview_res) => {
                let preview_res = crate::read_body_with_timeout(preview_res.into_body()).await?;
                let preview_res: JustContentHTML = serde_json::from_slice(&preview_res)?;

                page_community_new_post_inner(
                    community_id,
                    &req_parts.headers,
                    &cookies,
                    ctx,
                    None,
                    Some(&body_values_src),
                    Some(&preview_res.content_html),
                )
                .await
            }
            Err(crate::Error::RemoteError((_, message))) => {
                page_community_new_post_inner(
                    community_id,
                    &req_parts.headers,
                    &cookies,
                    ctx,
                    Some(message),
                    Some(&body_values_src),
                    None,
                )
                .await
            }
            Err(other) => Err(other),
        };
    }

    body_values.insert("community".into(), Cow::Owned(community_id.into()));
    if body_values.get("content_markdown").and_then(|x| x.as_str()) == Some("") {
        body_values.remove("content_markdown");
    }
    if body_values.get("href").and_then(|x| x.as_str()) == Some("") {
        body_values.remove("href");
    }

    if body_values.remove("sensitive").is_some() {
        body_values.insert("sensitive".into(), Cow::Owned(true.into()));
    }

    if body_values.remove("poll_enabled").is_some() {
        let options: Vec<_> = (0..4)
            .filter_map(|idx| {
                let value = body_values.remove(&*format!("poll_option_{idx}"));
                if value.as_ref().map(std::convert::AsRef::as_ref) == Some(&serde_json::json!("")) {
                    None
                } else {
                    value
                }
            })
            .collect();
        let multiple: bool = body_values.remove("poll_multiple").is_some();

        let duration_value = body_values.remove("poll_duration_value");
        let duration_value = duration_value.as_ref().and_then(|x| x.as_str()).ok_or(
            crate::Error::InternalStrStatic("Missing poll_duration_value"),
        )?;

        let duration_unit = body_values.remove("poll_duration_unit");
        let closed_in = match duration_unit.as_ref().and_then(|x| x.as_str()).ok_or(
            crate::Error::InternalStrStatic("Missing poll_duration_unit"),
        )? {
            "m" => format!("PT{duration_value}M"),
            "h" => format!("PT{duration_value}H"),
            "d" => format!("P{duration_value}D"),
            _ => return Err(crate::Error::InternalStrStatic("Unknown duration unit")),
        };

        body_values.insert(
            "poll".into(),
            Cow::Owned(serde_json::json!({
                "options": options,
                "multiple": multiple,
                "closed_in": closed_in,
            })),
        );
    }

    let api_res = res_to_error(
        ctx.http_client
            .request(for_client(
                hyper::Request::post(format!("{}/api/unstable/posts", ctx.backend_host))
                    .body(serde_json::to_vec(&body_values)?.into())?,
                &req_parts.headers,
                &cookies,
            )?)
            .await?,
    )
    .await;

    match api_res {
        Ok(api_res) => {
            #[derive(Deserialize)]
            struct PostsCreateResponse {
                id: i64,
            }

            let api_res = crate::read_body_with_timeout(api_res.into_body()).await?;
            let api_res: PostsCreateResponse = serde_json::from_slice(&api_res)?;

            Ok(cache_invalidating_response(
                hyper::Response::builder()
                    .status(hyper::StatusCode::SEE_OTHER)
                    .header(hyper::header::LOCATION, format!("/posts/{}", api_res.id))
                    .body("Successfully posted.".into())?,
            ))
        }
        Err(crate::Error::RemoteError((_, message))) => {
            page_community_new_post_inner(
                community_id,
                &req_parts.headers,
                &cookies,
                ctx,
                Some(message),
                Some(&body_values_src),
                None,
            )
            .await
        }
        Err(other) => Err(other),
    }
}

pub fn route_communities() -> crate::RouteNode<()> {
    crate::RouteNode::new()
        .with_handler_async(hyper::Method::GET, page_communities)
        .with_child(
            "unfollow_inactive",
            crate::RouteNode::new()
                .with_handler_async(hyper::Method::POST, handler_communities_unfollow_inactive),
        )
        .with_child_parse::<i64, _>(
            crate::RouteNode::new()
                .with_handler_async(hyper::Method::GET, page_community)
                .with_child(
                    "edit",
                    crate::RouteNode::new()
                        .with_handler_async(hyper::Method::GET, page_community_edit)
                        .with_child(
                            "submit",
                            crate::RouteNode::new().with_handler_async(
                                hyper::Method::POST,
                                handler_communities_edit_submit,
                            ),
                        ),
                )
                .with_child(
                    "delete",
                    crate::RouteNode::new()
                        .with_handler_async(hyper::Method::GET, page_community_delete)
                        .with_child(
                            "confirm",
                            crate::RouteNode::new().with_handler_async(
                                hyper::Method::POST,
                                handler_community_delete_confirm,
                            ),
                        ),
                )
                .with_child(
                    "follow",
                    crate::RouteNode::new()
                        .with_handler_async(hyper::Method::POST, handler_community_follow),
                )
                .with_child(
                    "moderators",
                    crate::RouteNode::new()
                        .with_handler_async(hyper::Method::GET, page_community_moderators)
                        .with_child(
                            "add",
                            crate::RouteNode::new().with_handler_async(
                                hyper::Method::POST,
                                handler_community_moderators_add,
                            ),
                        )
                        .with_child(
                            "remove",
                            crate::RouteNode::new().with_handler_async(
                                hyper::Method::POST,
                                handler_community_moderators_remove,
                            ),
                        ),
                )
                .with_child(
                    "modlog",
                    crate::RouteNode::new()
                        .with_handler_async(hyper::Method::GET, page_community_modlog),
                )
                .with_child(
                    "posts",
                    crate::RouteNode::new().with_child_parse::<i64, _>(
                        crate::RouteNode::new()
                            .with_child(
                                "approve",
                                crate::RouteNode::new().with_handler_async(
                                    hyper::Method::POST,
                                    handler_community_post_approve,
                                ),
                            )
                            .with_child(
                                "make_sticky",
                                crate::RouteNode::new().with_handler_async(
                                    hyper::Method::POST,
                                    handler_community_post_make_sticky,
                                ),
                            )
                            .with_child(
                                "make_unsticky",
                                crate::RouteNode::new().with_handler_async(
                                    hyper::Method::POST,
                                    handler_community_post_make_unsticky,
                                ),
                            )
                            .with_child(
                                "unapprove",
                                crate::RouteNode::new().with_handler_async(
                                    hyper::Method::POST,
                                    handler_community_post_unapprove,
                                ),
                            ),
                    ),
                )
                .with_child(
                    "unfollow",
                    crate::RouteNode::new()
                        .with_handler_async(hyper::Method::POST, handler_community_unfollow),
                )
                .with_child(
                    "new_post",
                    crate::RouteNode::new()
                        .with_handler_async(hyper::Method::GET, page_community_new_post)
                        .with_child(
                            "submit",
                            crate::RouteNode::new().with_handler_async(
                                hyper::Method::POST,
                                handler_communities_new_post_submit,
                            ),
                        ),
                ),
        )
}

#[cfg(test)]
mod tests {
    use crate::hyper;

    #[test]
    fn communities_page_numbers_keep_a_small_window() {
        assert_eq!(super::communities_page_numbers(1, false), vec![Some(1)]);
        assert_eq!(
            super::communities_page_numbers(1, true),
            vec![Some(1), Some(2)]
        );
        assert_eq!(
            super::communities_page_numbers(4, true),
            vec![Some(1), Some(2), Some(3), Some(4), Some(5)]
        );
        assert_eq!(
            super::communities_page_numbers(8, true),
            vec![Some(1), None, Some(6), Some(7), Some(8), Some(9)]
        );
    }

    #[test]
    fn communities_count_text_matches_scope_and_search() {
        assert_eq!(
            super::communities_count_text("mine", 1, false),
            "1 subscribed community"
        );
        assert_eq!(
            super::communities_count_text("mine", 2, false),
            "2 subscribed communities"
        );
        assert_eq!(
            super::communities_count_text("mine", 2, true),
            "2 matching subscribed communities"
        );
        assert_eq!(
            super::communities_count_text("everything", 1, false),
            "1 community"
        );
        assert_eq!(
            super::communities_count_text("everything", 2, true),
            "2 matching communities"
        );
        assert_eq!(
            super::communities_count_text("everything", -10, false),
            "0 communities"
        );
    }

    #[test]
    fn community_activity_text_uses_remote_count_without_local_preview() {
        assert_eq!(
            super::community_activity_text(Some("A visible post"), Some(50)),
            "Last post: A visible post"
        );
        assert_eq!(
            super::community_activity_text(None, Some(1)),
            "Remote reports 1 post"
        );
        assert_eq!(
            super::community_activity_text(None, Some(42)),
            "Remote reports 42 posts"
        );
        assert_eq!(super::community_activity_text(None, None), "No posts seen");
    }

    #[test]
    fn community_action_return_location_accepts_only_communities_pages() {
        let req = hyper::Request::builder()
            .header(
                hyper::header::REFERER,
                "https://lotide.example/communities?scope=everything&page=2",
            )
            .body(hyper::Body::empty())
            .unwrap();

        assert_eq!(
            super::community_action_return_location(&req, "/communities/1".to_owned()),
            "/communities?scope=everything&page=2"
        );

        let req = hyper::Request::builder()
            .header(
                hyper::header::REFERER,
                "https://lotide.example/communities_evil?return=/communities",
            )
            .body(hyper::Body::empty())
            .unwrap();

        assert_eq!(
            super::community_action_return_location(&req, "/communities/1".to_owned()),
            "/communities/1"
        );
    }

    #[test]
    fn communities_pagination_normalizes_bad_current_pages() {
        let pagination =
            super::render_communities_pagination(None, "mine", -4, true, None, None).unwrap();

        assert!(matches!(
            pagination.items.as_slice(),
            [
                super::CommunitiesPaginationItem::Page {
                    page: 1,
                    href: None,
                    current: true
                },
                super::CommunitiesPaginationItem::Page {
                    page: 2,
                    href: Some(_),
                    current: false
                }
            ]
        ));
    }

    #[test]
    fn communities_url_preserves_active_filters_and_omits_defaults() {
        assert_eq!(
            super::communities_url(
                Some("lemmy.world"),
                "everything",
                Some(2),
                Some("lemmy"),
                Some("host")
            )
            .unwrap(),
            "/communities?search=lemmy.world&scope=everything&page=2&software=lemmy&sort=host"
        );

        assert_eq!(
            super::communities_url(None, "mine", None, Some("all"), Some("alphabetic")).unwrap(),
            "/communities?scope=mine"
        );
    }

    #[test]
    fn community_filter_normalizers_reject_unknown_ui_values_to_defaults() {
        assert_eq!(
            super::normalize_community_software(Some("peertube")),
            "peertube"
        );
        assert_eq!(
            super::normalize_community_software(Some("buzzrelay")),
            "buzzrelay"
        );
        assert_eq!(super::normalize_community_software(Some("bad")), "all");
        assert_eq!(
            super::normalize_community_sort(Some("post_count")),
            "post_count"
        );
        assert_eq!(super::normalize_community_sort(Some("bad")), "alphabetic");
    }

    #[test]
    fn community_software_label_keeps_unidentified_hosts_neutral() {
        assert_eq!(
            super::community_software_label("unknown").as_ref(),
            "Unclassified"
        );
        assert_eq!(
            super::community_software_label("buzzrelay").as_ref(),
            "BuzzRelay"
        );
    }
}
