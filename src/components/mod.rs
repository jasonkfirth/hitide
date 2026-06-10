pub mod timeago;

use std::borrow::{Borrow, Cow};
use std::collections::HashMap;

use crate::PageBaseData;
use crate::lang;
use crate::resp_types::{
    Content, RespCommentInfo, RespFederationStatus, RespFlagDetails, RespFlagInfo,
    RespMinimalAuthorInfo, RespMinimalCommentInfo, RespMinimalCommunityInfo, RespNotification,
    RespNotificationInfo, RespPollInfo, RespPostCommentInfo, RespPostInfo, RespPostListPost,
    RespSiteModlogEvent, RespSiteModlogEventDetails, RespThingComment, RespThingInfo,
    RespYourFollow,
};
use crate::util::{abbreviate_link, author_is_me, safe_href};

pub use timeago::SafeTimeAgo;

fn federation_status_text(status: RespFederationStatus) -> &'static str {
    match status {
        RespFederationStatus::Unsent => "federation: unsent",
        RespFederationStatus::Sent => "federation: sent",
        RespFederationStatus::Received => "federation: received",
        RespFederationStatus::Posted => "federation: posted",
    }
}

fn federation_status_class(status: RespFederationStatus) -> &'static str {
    match status {
        RespFederationStatus::Unsent => "federationStatus federationStatusUnsent",
        RespFederationStatus::Sent => "federationStatus federationStatusSent",
        RespFederationStatus::Received => "federationStatus federationStatusReceived",
        RespFederationStatus::Posted => "federationStatus federationStatusPosted",
    }
}

/*
    Hitide does not infer federation state from page context. The backend owns
    the delivery lifecycle, and the frontend only renders that state beside the
    action or object that produced it.
*/
pub fn federation_status_line_class(status: Option<RespFederationStatus>) -> &'static str {
    match status {
        Some(RespFederationStatus::Unsent) => "federationStatusLine federationStatusLineUnsent",
        Some(RespFederationStatus::Sent) => "federationStatusLine federationStatusLineSent",
        Some(RespFederationStatus::Received) => "federationStatusLine federationStatusLineReceived",
        Some(RespFederationStatus::Posted) => "federationStatusLine federationStatusLinePosted",
        None => "",
    }
}

#[derive(Clone, Copy)]
pub struct FederationStatusBadge {
    pub status: Option<RespFederationStatus>,
}

impl render::Render for FederationStatusBadge {
    fn render_into<W: std::fmt::Write + ?Sized>(self, w: &mut W) -> std::fmt::Result {
        if let Some(status) = self.status {
            render::rsx! {
                <span class={federation_status_class(status)} title={"Federation status"}>
                    {federation_status_text(status)}
                </span>
            }
            .render_into(w)
        } else {
            Ok(())
        }
    }
}

fn follow_federation_status_badge_data(
    your_follow: Option<&RespYourFollow>,
    latest_unfollow_status: Option<RespFederationStatus>,
) -> Option<(RespFederationStatus, &'static str)> {
    /*
        A plain "federation: received" label is not enough for follows. A
        remote inbox can accept the request before the group or user sends the
        ActivityPub Accept that makes the follow usable.
    */
    if let Some(follow) = your_follow {
        return match follow.federation_status {
            Some(RespFederationStatus::Unsent) => {
                Some((RespFederationStatus::Unsent, "follow request: queued"))
            }
            Some(RespFederationStatus::Sent) => {
                Some((RespFederationStatus::Sent, "follow request: sent"))
            }
            Some(RespFederationStatus::Received) => Some((
                RespFederationStatus::Received,
                "follow request: received by remote",
            )),
            Some(RespFederationStatus::Posted) => {
                Some((RespFederationStatus::Posted, "follow request: accepted"))
            }
            None if !follow.accepted => {
                Some((RespFederationStatus::Unsent, "follow request: pending"))
            }
            None => None,
        };
    }

    latest_unfollow_status.map(|status| {
        let label = match status {
            RespFederationStatus::Unsent => "unfollow: queued",
            RespFederationStatus::Sent => "unfollow: sent",
            RespFederationStatus::Received | RespFederationStatus::Posted => {
                "unfollow: received by remote"
            }
        };

        (status, label)
    })
}

#[derive(Clone, Copy)]
pub struct FollowFederationStatusBadge<'a> {
    pub your_follow: Option<&'a RespYourFollow>,
    pub latest_unfollow_status: Option<RespFederationStatus>,
}

impl render::Render for FollowFederationStatusBadge<'_> {
    fn render_into<W: std::fmt::Write + ?Sized>(self, w: &mut W) -> std::fmt::Result {
        if let Some((status, label)) =
            follow_federation_status_badge_data(self.your_follow, self.latest_unfollow_status)
        {
            render::rsx! {
                <span class={federation_status_class(status)} title={"Follow federation status"}>
                    {label}
                </span>
            }
            .render_into(w)
        } else {
            Ok(())
        }
    }
}

#[render::component]
pub fn Comment<'a>(
    comment: &'a RespPostCommentInfo<'a>,
    sort: crate::SortType,
    root_sensitive: bool,
    base_data: &'a PageBaseData,
    lang: &'a crate::Translator,
    interactions_blocked: bool,
) {
    let sensitive_hide = !root_sensitive && comment.as_ref().sensitive;
    let vote_status = comment
        .your_vote
        .as_ref()
        .and_then(|vote| vote.federation_status);
    let vote_class = format!("votebox {}", federation_status_line_class(vote_status));

    render::rsx! {
        <li class={"comment"} id={format!("comment{}", comment.as_ref().id)}>
            {
                if base_data.login.is_some() {
                    Some(render::rsx! {
                        <div class={vote_class}>
                            {
                                if comment.your_vote.is_some() {
                                    render::rsx! {
                                        <form method={"POST"} action={format!("/comments/{}/unlike", comment.as_ref().id)}>
                                            <button class={"iconbutton"} type={"submit"}>{hitide_icons::UPVOTED.img(lang.tr(&lang::remove_upvote()).into_owned())}</button>
                                        </form>
                                    }
                                } else if interactions_blocked {
                                    render::rsx! {
                                        <form method={"POST"} action={"#"}>
                                            <button class={"iconbutton"} type={"submit"} disabled={"disabled"}>{hitide_icons::UPVOTE.img(lang.tr(&lang::upvote()).into_owned())}</button>
                                        </form>
                                    }
                                } else {
                                    render::rsx! {
                                        <form method={"POST"} action={format!("/comments/{}/like", comment.as_ref().id)}>
                                            <button class={"iconbutton"} type={"submit"}>{hitide_icons::UPVOTE.img(lang.tr(&lang::upvote()).into_owned())}</button>
                                        </form>
                                    }
                                }
                            }
                            <FederationStatusBadge status={vote_status} />
                        </div>
                    })
                } else {
                    None
                }
            }
            <details class={"commentCollapse"} open={"open"}>
                <summary>
                    <small>
                        <cite><UserLink lang user={comment.author.as_ref()} /></cite>
                        {" "}
                        <SafeTimeAgo since={comment.created.as_ref()} lang />
                        {" "}
                        <FederationStatusBadge status={comment.federation_status} />
                    </small>
                </summary>
                <div class={"content"}>
                    <div class={"commentContent"}>
                        {
                            sensitive_hide.then(|| {
                                render::rsx! {
                                    <details>
                                        <summary>
                                            {hitide_icons::SENSITIVE.img_aria_hidden()}
                                            {lang.tr(&lang::SENSITIVE)}
                                        </summary>
                                        <ContentView src={comment} />
                                    </details>
                                }
                            })
                        }
                        {
                            (!sensitive_hide).then_some({
                                render::rsx! { <ContentView src={comment} /> }
                            })
                        }
                    </div>
                    {
                        comment.attachments.iter().map(|attachment| {
                            let href = safe_href(&attachment.url).unwrap_or("#");
                            render::rsx! {
                                <div>
                                    <strong>{lang.tr(&lang::COMMENT_ATTACHMENT_PREFIX)}</strong>
                                    {" "}
                                    <em><a href={href}>{abbreviate_link(href)}{" ↗"}</a></em>
                                </div>
                            }
                        })
                        .collect::<Vec<_>>()
                    }
                    <div class={"actionList small"}>
                        {
                            if base_data.login.is_some() && !interactions_blocked {
                                Some(render::rsx! {
                                    <a href={format!("/comments/{}?sort={}", comment.as_ref().id, sort.as_str())}>{lang.tr(&lang::REPLY)}</a>
                                })
                            } else {
                                None
                            }
                        }
                        {
                            if comment.local {
                                None
                            } else {
                                comment.as_ref().remote_url.as_ref().map(|remote_url| render::rsx! {
                                    <a href={safe_href(remote_url).unwrap_or("#")}>{lang.tr(&lang::remote_url()).into_owned()}</a>
                                })
                            }
                        }
                        {
                            if author_is_me(&comment.author, &base_data.login) || (comment.local && base_data.is_site_admin()) {
                                Some(render::rsx! {
                                    <a href={format!("/comments/{}/delete", comment.as_ref().id)}>{lang.tr(&lang::DELETE)}</a>
                                })
                            } else {
                                None
                            }
                        }
                    </div>
                </div>

                {
                    if let Some(replies) = &comment.replies {
                        if replies.items.is_empty() {
                            None
                        } else {
                            Some(render::rsx! {
                                <>
                                    <ul class={"commentList"}>
                                        {
                                            replies.items.iter().map(|reply| {
                                                render::rsx! {
                                                    <Comment sort={sort} comment={reply} root_sensitive base_data lang interactions_blocked={interactions_blocked} />
                                                }
                                            })
                                            .collect::<Vec<_>>()
                                        }
                                    </ul>
                                    {
                                        replies.next_page.as_ref().map(|next_page| {
                                            render::rsx! {
                                                <a href={format!("/comments/{}?sort={}&page={}", comment.base.id, sort.as_str(), next_page)}>{"-> "}{lang.tr(&lang::VIEW_MORE_COMMENTS)}</a>
                                            }
                                        })
                                    }
                                </>
                            })
                        }
                    } else {
                        None
                    }
                }
                {
                    if comment.replies.is_none() {
                        Some(render::rsx! {
                            <ul><li><a href={format!("/comments/{}", comment.as_ref().id)}>{"-> "}{lang.tr(&lang::VIEW_MORE_COMMENTS)}</a></li></ul>
                        })
                    } else {
                        None
                    }
                }
            </details>
        </li>
    }
}

pub struct CommunityLink<'community> {
    pub community: &'community RespMinimalCommunityInfo<'community>,
}
impl render::Render for CommunityLink<'_> {
    fn render_into<W: std::fmt::Write + ?Sized>(self, writer: &mut W) -> std::fmt::Result {
        let community = &self.community;

        if community.deleted {
            (render::rsx! {
                <strong>{"[deleted]"}</strong>
            })
            .render_into(writer)
        } else {
            let href = format!("/communities/{}", community.id);
            (render::rsx! {
                <a href={&href}>
                {
                    (if community.local {
                        community.name.as_ref().into()
                    } else {
                        Cow::Owned(format!("{}@{}", community.name, community.host))
                    }).as_ref()
                }
                </a>
            })
            .render_into(writer)
        }
    }
}

pub trait HavingContent {
    fn content_text(&self) -> Option<&str>;
    fn content_html(&self) -> Option<&str>;
}

impl HavingContent for RespMinimalCommentInfo<'_> {
    fn content_text(&self) -> Option<&str> {
        self.content_text.as_deref()
    }
    fn content_html(&self) -> Option<&str> {
        self.content_html.as_deref()
    }
}

impl HavingContent for RespThingComment<'_> {
    fn content_text(&self) -> Option<&str> {
        self.base.content_text()
    }
    fn content_html(&self) -> Option<&str> {
        self.base.content_html()
    }
}

impl HavingContent for RespPostCommentInfo<'_> {
    fn content_text(&self) -> Option<&str> {
        self.base.content_text()
    }
    fn content_html(&self) -> Option<&str> {
        self.base.content_html()
    }
}

impl HavingContent for RespCommentInfo<'_> {
    fn content_text(&self) -> Option<&str> {
        self.base.content_text()
    }
    fn content_html(&self) -> Option<&str> {
        self.base.content_html()
    }
}

impl HavingContent for RespPostInfo<'_> {
    fn content_text(&self) -> Option<&str> {
        self.content_text.as_deref()
    }
    fn content_html(&self) -> Option<&str> {
        self.content_html.as_deref()
    }
}

impl HavingContent for Content<'_> {
    fn content_text(&self) -> Option<&str> {
        self.content_text.as_deref()
    }
    fn content_html(&self) -> Option<&str> {
        self.content_html.as_deref()
    }
}

#[derive(Clone)]
pub struct HavingContentRef<'a> {
    content_html: Option<&'a str>,
    content_text: Option<&'a str>,
}

impl HavingContent for HavingContentRef<'_> {
    fn content_text(&self) -> Option<&str> {
        self.content_text
    }
    fn content_html(&self) -> Option<&str> {
        self.content_html
    }
}

#[derive(Clone)]
pub struct ContentView<'a, T: HavingContent + 'a> {
    pub src: &'a T,
}

impl<'a, T: HavingContent + 'a> render::Render for ContentView<'a, T> {
    fn render_into<W: std::fmt::Write + ?Sized>(self, writer: &mut W) -> std::fmt::Result {
        match self.src.content_html() {
            Some(html) => {
                (render::rsx! { <div class={"contentView"}>{render::raw!(html)}</div> })
                    .render_into(writer)?;
            }
            None => {
                if let Some(text) = self.src.content_text() {
                    (render::rsx! { <div class={"contentView"}>{text}</div> })
                        .render_into(writer)?;
                }
            }
        }

        Ok(())
    }
}

pub struct FlagItem<'a> {
    pub flag: &'a RespFlagInfo<'a>,
    pub in_community: bool,
    pub lang: &'a crate::Translator,
}

impl render::Render for FlagItem<'_> {
    fn render_into<W: std::fmt::Write + ?Sized>(self, w: &mut W) -> std::fmt::Result {
        let Self {
            flag,
            in_community,
            lang,
        } = self;

        let RespFlagDetails::Post { post } = &flag.details;

        render::rsx! {
            <li class={"flagItem"}>
                <div class={"flaggedContent"}>
                    <PostItemContent post={post} in_community no_user={false} lang />
                </div>
                {
                    lang::TrElements::new(
                        lang.tr(&lang::flagged_by(lang::LangPlaceholder(0))),
                        |id, w| {
                            match id {
                                0 => render::rsx! {
                                    <UserLink user={Some(&flag.flagger)} lang />
                                }.render_into(w),
                                _ => unreachable!(),
                            }
                        },
                    )
                }
                {
                    flag.content.as_ref().map(|content| {
                        render::rsx! {
                            <blockquote>
                                {content.content_text.as_ref()}
                            </blockquote>
                        }
                    })
                }
            </li>
        }
        .render_into(w)
    }
}

#[render::component]
pub fn HTPage<'a, Children: render::Render>(
    base_data: &'a PageBaseData,
    lang: &'a crate::Translator,
    title: &'a str,
    children: Children,
) {
    render::rsx! {
        <HTPageAdvanced base_data={base_data} lang={lang} title={title} head_items={()}>{children}</HTPageAdvanced>
    }
}

pub fn site_stylesheet_href(base_data: &PageBaseData) -> &str {
    base_data
        .site_css_url
        .as_deref()
        .unwrap_or("/static/main.css")
}

#[render::component]
pub fn HTPageAdvanced<'a, HeadItems: render::Render, Children: render::Render>(
    base_data: &'a PageBaseData,
    lang: &'a crate::Translator,
    title: &'a str,
    head_items: HeadItems,
    children: Children,
) {
    let left_links = render::rsx! {
        <>
            <a href={"/all"}>{lang.tr(&lang::ALL)}</a>
            <a href={"/local"}>{lang.tr(&lang::LOCAL)}</a>
            <a href={"/communities"}>{lang.tr(&lang::COMMUNITIES)}</a>
            <a href={"/about"}>{lang.tr(&lang::ABOUT)}</a>
        </>
    };
    let document_title = if title == base_data.site_name.as_str() {
        title.to_owned()
    } else {
        format!("{} - {}", title, base_data.site_name)
    };

    render::rsx! {
        <>
            <render::html::HTML5Doctype />
            <html
                lang={lang.primary_language().to_string()}
                dir={match lang.primary_language().character_direction() {
                    unic_langid::CharacterDirection::LTR => "ltr",
                    unic_langid::CharacterDirection::RTL => "rtl",
                    unic_langid::CharacterDirection::TTB => "ttb"
                 }}
            >
                <head>
                    <meta charset={"utf-8"} />
                    <meta name={"viewport"} content={"width=device-width, initial-scale=1"} />
                    <link rel={"stylesheet"} href={safe_href(site_stylesheet_href(base_data)).unwrap_or("/static/main.css")} />
                    {
                        base_data.site_logo_url.as_ref().map(|site_logo_url| {
                            render::rsx! {
                                <link rel={"icon"} href={safe_href(site_logo_url).unwrap_or("")} />
                            }
                        })
                    }
                    <title>{document_title}</title>
                    {head_items}
                </head>
                <body>
                    <header class={"mainHeader"}>
                        <nav aria-label={"Main Navigation"} class={"left"}>
                            <details class={"leftLinksMobile"}>
                                <summary>{hitide_icons::HAMBURGER_MENU.img(lang.tr(&lang::open_menu()).into_owned())}</summary>
                                <div>
                                    {left_links.clone()}
                                </div>
                            </details>
                            <a href={"/"} class={"siteName"}>
                                {
                                    base_data.site_logo_url.as_ref().map(|site_logo_url| {
                                        render::rsx! {
                                            <img src={safe_href(site_logo_url).unwrap_or("")} class={"siteLogo"} alt={""} />
                                        }
                                    })
                                }
                                {base_data.site_name.as_str()}
                            </a>
                            <div class={"actionList leftLinks"}>
                                {left_links}
                            </div>
                        </nav>
                        <nav class={"right actionList"}>
                            {
                                base_data.login.as_ref().map(|login| {
                                    render::rsx! {
                                        <>
                                            <a
                                                href={"/notifications"}
                                            >
                                                {
                                                    if login.user.has_unread_notifications {
                                                        hitide_icons::NOTIFICATIONS_SOME.img(lang.tr(&lang::new_notifications()).into_owned())
                                                    } else {
                                                        hitide_icons::NOTIFICATIONS.img(lang.tr(&lang::notifications()).into_owned())
                                                    }
                                                }
                                            </a>
                                            <a href={format!("/users/{}", login.user.id)}>
                                                {hitide_icons::PERSON.img(lang.tr(&lang::profile()).into_owned())}
                                            </a>
                                            <a href={"/moderation"}>
                                                {
                                                    if login.user.has_pending_moderation_actions {
                                                        hitide_icons::MODERATION_SOME.img(lang.tr(&lang::moderation_dashboard_some()).into_owned())
                                                    } else {
                                                        hitide_icons::MODERATION.img(lang.tr(&lang::moderation_dashboard()).into_owned())
                                                    }
                                                }
                                            </a>
                                            {
                                                base_data.is_site_admin().then(|| {
                                                    render::rsx! {
                                                        <>
                                                            <a href={"/administration"}>
                                                                {hitide_icons::ADMINISTRATION.img(lang.tr(&lang::administration()).into_owned())}
                                                            </a>
                                                            <a href={"/flags?to_this_site_admin=true"}>
                                                                {hitide_icons::FLAG.img(lang.tr(&lang::flags()).into_owned())}
                                                            </a>
                                                        </>
                                                    }
                                                })
                                            }
                                            <form method={"POST"} action={"/logout"} class={"inline"}>
                                                <button type={"submit"} class={"iconbutton"}>
                                                    {hitide_icons::LOGOUT.img(lang.tr(&lang::logout()).into_owned())}
                                                </button>
                                            </form>
                                        </>
                                    }
                                })
                            }
                            {
                                if base_data.login.is_none() {
                                    Some(render::rsx! {
                                        <a href={"/login"}>{lang.tr(&lang::LOGIN)}</a>
                                    })
                                } else {
                                    None
                                }
                            }
                        </nav>
                    </header>
                    <main>
                        {children}
                    </main>
                </body>
            </html>
        </>
    }
}

#[render::component]
pub fn PostItem<'a>(
    post: &'a RespPostListPost<'a>,
    in_community: bool,
    no_user: bool,
    lang: &'a crate::Translator,
) {
    render::rsx! {
        <li class={if post.as_ref().sticky { "sticky" } else { "" }}>
            <PostItemContent post in_community no_user lang />
        </li>
    }
}

pub struct PostItemContent<'a> {
    post: &'a RespPostListPost<'a>,
    in_community: bool,
    no_user: bool,
    lang: &'a crate::Translator,
}

impl render::Render for PostItemContent<'_> {
    fn render_into<W: std::fmt::Write + ?Sized>(self, w: &mut W) -> std::fmt::Result {
        let Self {
            post,
            in_community,
            no_user,
            lang,
        } = self;

        let post_href = format!("/posts/{}", post.as_ref().as_ref().id);

        render::rsx! {
            <>
                <div class={"titleLine"}>
                    <a href={post_href.clone()}>
                        {post.as_ref().as_ref().sensitive.then(|| hitide_icons::SENSITIVE.img(lang.tr(&lang::SENSITIVE)))}
                        {post.as_ref().as_ref().title.as_ref()}
                    </a>
                    {
                        post.as_ref().href.as_ref().map(|href| {
                            let href = safe_href(href).unwrap_or("#");
                            render::rsx! {
                                <em><a href={href}>{abbreviate_link(href)}{" ↗"}</a></em>
                            }
                        })
                    }
                </div>
                <small>
                    {
                        lang::TrElements::new(
                            lang.tr(&match (no_user, in_community) {
                                (false, false) => lang::post_submitted_by_to(lang::LangPlaceholder(0), lang::LangPlaceholder(1), lang::LangPlaceholder(2)),
                                (false, true) => lang::post_submitted_by(lang::LangPlaceholder(0), lang::LangPlaceholder(1)),
                                (true, false) => lang::post_submitted_to(lang::LangPlaceholder(0), lang::LangPlaceholder(2)),
                                (true, true) => lang::post_submitted(lang::LangPlaceholder(0)),
                            }),
                            |id, w| {
                                match id {
                                    0 => render::rsx! {
                                        <SafeTimeAgo since={post.as_ref().created.as_ref()} lang />
                                    }.render_into(w),
                                    1 => render::rsx! {
                                        <UserLink lang user={post.as_ref().author.as_ref()} />
                                    }.render_into(w),
                                    2 => render::rsx! {
                                        <CommunityLink community={&post.as_ref().community} />
                                    }.render_into(w),
                                    _ => unreachable!(),
                                }
                            },
                        )
                    }
                    {" | "}
                    <a href={post_href}>{lang.tr(&lang::post_comments_count(post.replies_count_total)).into_owned()}</a>
                    {" "}
                    <FederationStatusBadge status={post.as_ref().federation_status} />
                </small>
            </>
        }.render_into(w)
    }
}

pub struct ThingItem<'a> {
    pub lang: &'a crate::Translator,
    pub thing: &'a RespThingInfo<'a>,
}

impl render::Render for ThingItem<'_> {
    fn render_into<W: std::fmt::Write + ?Sized>(self, writer: &mut W) -> std::fmt::Result {
        let lang = self.lang;

        match self.thing {
            RespThingInfo::Post(post) => {
                (PostItem { post, in_community: false, no_user: true, lang: self.lang }).render_into(writer)
            },
            RespThingInfo::Comment(comment) => {
                (render::rsx! {
                    <li>
                        <small>
                            {
                                lang::TrElements::new(
                                    lang.tr(&lang::thing_comment(lang::LangPlaceholder(0), lang::LangPlaceholder(1), lang::LangPlaceholder(2))),
                                    |id, w| {
                                        match id {
                                            0 => render::rsx! {
                                                <a href={format!("/comments/{}", comment.as_ref().id)}>
                                                    {lang.tr(&lang::thing_comment_part_comment())}
                                                </a>
                                            }.render_into(w),
                                            1 => render::rsx! {
                                                <a href={format!("/posts/{}", comment.post.id)}>
                                                    {comment.post.title.as_ref()}
                                                </a>
                                            }.render_into(w),
                                            2 => SafeTimeAgo {
                                                since: comment.created.as_ref(),
                                                lang,
                                            }.render_into(w),
                                            _ => unreachable!(),
                                        }
                                    }
                                )
                            }
                            {" "}
                            <FederationStatusBadge status={comment.federation_status} />
                        </small>
                        <ContentView src={comment} />
                    </li>
                }).render_into(writer)
            }
        }
    }
}

pub struct UserLink<'a> {
    pub lang: &'a crate::Translator,
    pub user: Option<&'a RespMinimalAuthorInfo<'a>>,
}

pub struct UserAvatar<'a> {
    pub user: &'a RespMinimalAuthorInfo<'a>,
    pub class_name: &'static str,
    pub fallback: bool,
}

impl render::Render for UserAvatar<'_> {
    fn render_into<W: std::fmt::Write + ?Sized>(self, writer: &mut W) -> std::fmt::Result {
        if let Some(avatar) = &self.user.avatar {
            render::rsx! {
                <img
                    src={safe_href(avatar.url.as_ref()).unwrap_or("")}
                    class={self.class_name}
                    alt={""}
                    loading={"lazy"}
                />
            }
            .render_into(writer)
        } else if self.fallback {
            let initial = self
                .user
                .username
                .chars()
                .next()
                .map_or_else(|| "?".to_owned(), |ch| ch.to_uppercase().to_string());
            let class_name = format!("{} userAvatarPlaceholder", self.class_name);

            render::rsx! {
                <span class={class_name} aria-hidden={"true"}>{initial}</span>
            }
            .render_into(writer)
        } else {
            Ok(())
        }
    }
}

impl render::Render for UserLink<'_> {
    fn render_into<W: std::fmt::Write + ?Sized>(self, writer: &mut W) -> std::fmt::Result {
        match self.user {
            None => "[unknown]".render_into(writer),
            Some(user) => {
                let href = format!("/users/{}", user.id);
                (render::rsx! {
                    <a href={&href} class={"userLink"}>
                        <UserAvatar user class_name={"userAvatarSmall"} fallback={false} />
                        {
                            (if user.local {
                                user.username.as_ref().into()
                            } else {
                                Cow::Owned(format!("{}@{}", user.username, user.host))
                            }).as_ref()
                        }
                        {
                            if user.is_bot {
                                Some(format!(" [{}]", self.lang.tr(&lang::user_bot_tag())))
                            } else {
                                None
                            }
                        }
                    </a>
                })
                .render_into(writer)
            }
        }
    }
}

pub trait GetIndex<K, V> {
    fn get(&self, key: K) -> Option<&V>;
}

impl<K: Borrow<Q> + Eq + std::hash::Hash, V, Q: ?Sized + Eq + std::hash::Hash> GetIndex<&Q, V>
    for HashMap<K, V>
{
    fn get<'a>(&'a self, key: &Q) -> Option<&'a V> {
        HashMap::get(self, key)
    }
}

impl<I: serde_json::value::Index> GetIndex<I, serde_json::Value> for serde_json::Value {
    fn get(&self, key: I) -> Option<&serde_json::Value> {
        self.get(key)
    }
}

pub fn maybe_fill_value<'a, 'b, M: GetIndex<&'b str, serde_json::Value>>(
    values: &'a Option<&'a M>,
    name: &'b str,
    default_value: Option<&'a str>,
) -> &'a str {
    values
        .and_then(|values| values.get(name))
        .and_then(serde_json::Value::as_str)
        .or(default_value)
        .unwrap_or("")
}

#[render::component]
pub fn MaybeFillInput<'a, M: GetIndex<&'a str, serde_json::Value>>(
    values: &'a Option<&'a M>,
    r#type: &'a str,
    name: &'a str,
    required: bool,
    id: &'a str,
) {
    let value = maybe_fill_value(values, name, None);
    if required {
        render::rsx! {
            <input
                r#type
                name
                value
                id
                required={""}
            />
        }
    } else {
        render::rsx! {
            <input
                r#type
                name
                value
                id
            />
        }
    }
}

#[render::component]
pub fn MaybeFillCheckbox<'a, M: GetIndex<&'a str, serde_json::Value>>(
    values: &'a Option<&'a M>,
    name: &'a str,
    id: &'a str,
    default: bool,
) {
    let checked = values
        .and_then(|x| x.get(name))
        .map_or(default, |x| x.as_bool().unwrap_or(true));
    log::debug!(
        "MaybeFillCheckbox {} checked={} (values? {})",
        name,
        checked,
        values.is_some()
    );
    if checked {
        render::rsx! {
            <input
                type={"checkbox"}
                name
                id
                checked={""}
            />
        }
    } else {
        render::rsx! {
            <input
                type={"checkbox"}
                name
                id
            />
        }
    }
}

#[render::component]
pub fn MaybeFillOption<'a, M: GetIndex<&'a str, serde_json::Value>, Children: render::Render>(
    values: &'a Option<&'a M>,
    default_value: Option<&'a str>,
    name: &'a str,
    value: &'a str,
    children: Children,
) {
    let selected_value = maybe_fill_value(values, name, default_value);

    SelectOption {
        value,
        selected: selected_value == value,
        children,
    }
}

#[render::component]
pub fn SelectOption<'a, Children: render::Render>(
    value: &'a str,
    selected: bool,
    children: Children,
) {
    if selected {
        render::rsx! {
            <option value={value} selected={""}>{children}</option>
        }
    } else {
        render::rsx! {
            <option value={value}>{children}</option>
        }
    }
}

#[render::component]
pub fn MaybeFillTextArea<'a, M: GetIndex<&'a str, serde_json::Value>>(
    values: &'a Option<&'a M>,
    name: &'a str,
    default_value: Option<&'a str>,
) {
    render::rsx! {
        <textarea name>
            {maybe_fill_value(values, name, default_value)}
        </textarea>
    }
}

#[render::component]
pub fn BoolSubmitButton<'a>(value: bool, do_text: &'a str, done_text: &'a str) {
    if value {
        render::rsx! {
            <button disabled={""}>{done_text}</button>
        }
    } else {
        render::rsx! {
            <button type={"submit"}>{do_text}</button>
        }
    }
}

#[render::component]
pub fn BoolCheckbox<'a>(name: &'a str, value: bool) {
    if value {
        render::rsx! {
            <input name type={"checkbox"} checked={""} />
        }
    } else {
        render::rsx! {
            <input type={"checkbox"} name />
        }
    }
}

pub struct NotificationItem<'a> {
    pub notification: &'a RespNotification<'a>,
    pub lang: &'a crate::Translator,
}

impl render::Render for NotificationItem<'_> {
    fn render_into<W: std::fmt::Write + ?Sized>(self, writer: &mut W) -> std::fmt::Result {
        let lang = self.lang;

        write!(writer, "<li class=\"notification-item")?;
        if self.notification.unseen {
            write!(writer, " unread")?;
        }
        write!(writer, "\">")?;
        match &self.notification.info {
            RespNotificationInfo::Unknown => {
                "[unknown notification type]".render_into(writer)?;
            }
            RespNotificationInfo::PostReply { reply, post } => {
                (render::rsx! {
                    <>
                        <div>
                            {
                                lang::TrElements::new(
                                    lang.tr(&lang::notification_post_reply(lang::LangPlaceholder(0), lang::LangPlaceholder(1))),
                                    |id, w| {
                                        match id {
                                            0 => render::rsx! {
                                                <a href={format!("/comments/{}", reply.as_ref().id)}>
                                                    {lang.tr(&lang::notification_post_reply_part_comment())}
                                                </a>
                                            }.render_into(w),
                                            1 => render::rsx! {
                                                <a href={format!("/posts/{}", post.as_ref().as_ref().id)}>
                                                    {post.as_ref().as_ref().title.as_ref()}
                                                </a>
                                            }.render_into(w),
                                            _ => unreachable!(),
                                        }
                                    }
                                )
                            }
                        </div>
                        <div class={"body"}>
                            <small>
                                <cite><UserLink lang user={reply.author.as_ref()} /></cite>
                                {" "}
                                <SafeTimeAgo since={reply.created.as_ref()} lang />
                            </small>
                            <ContentView src={reply} />
                        </div>
                    </>
                }).render_into(writer)?;
            }
            RespNotificationInfo::PostMention { post } => {
                (render::rsx! {
                    <>
                        <div>
                            {lang.tr(&lang::notification_post_mention())}
                            <div class={"body"}>
                                <PostItemContent post={post} in_community={false} no_user={false} lang={lang} />
                            </div>
                        </div>
                    </>
                }).render_into(writer)?;
            }
            RespNotificationInfo::CommentReply {
                reply,
                comment,
                post,
            } => {
                (render::rsx! {
                    <>
                        <div>
                            {
                                lang::TrElements::new(
                                    lang.tr(&lang::notification_comment_reply(lang::LangPlaceholder(0), lang::LangPlaceholder(1))),
                                    |id, w| {
                                        match id {
                                            0 => render::rsx! {
                                                <a href={format!("/comments/{}", comment.as_ref().id)}>
                                                    {lang.tr(&lang::notification_comment_reply_part_your_comment())}
                                                </a>
                                            }.render_into(w),
                                            1 => render::rsx! {
                                                <a href={format!("/posts/{}", post.as_ref().as_ref().id)}>
                                                    {post.as_ref().as_ref().title.as_ref()}
                                                </a>
                                            }.render_into(w),
                                            _ => unreachable!(),
                                        }
                                    }
                                )
                            }
                        </div>
                        <div class={"body"}>
                            <small>
                                <cite><UserLink lang user={reply.author.as_ref()} /></cite>
                                {" "}
                                <SafeTimeAgo since={reply.created.as_ref()} lang />
                            </small>
                            <ContentView src={reply} />
                        </div>
                    </>
                }).render_into(writer)?;
            }
            RespNotificationInfo::CommentMention { comment, post } => {
                (render::rsx! {
                    <>
                        <div>
                            {
                                lang::TrElements::new(
                                    lang.tr(&lang::notification_comment_mention(lang::LangPlaceholder(0), lang::LangPlaceholder(1))),
                                    |id, w| {
                                        match id {
                                            0 => render::rsx! {
                                                <a href={format!("/comments/{}", comment.as_ref().id)}>
                                                    {lang.tr(&lang::notification_comment_mention_part_comment())}
                                                </a>
                                            }.render_into(w),
                                            1 => render::rsx! {
                                                <a href={format!("/posts/{}", post.as_ref().as_ref().id)}>
                                                    {post.as_ref().as_ref().title.as_ref()}
                                                </a>
                                            }.render_into(w),
                                            _ => unreachable!(),
                                        }
                                    },
                                )
                            }
                            <div class={"body"}>
                                <small>
                                    <cite><UserLink lang user={comment.author.as_ref()} /></cite>
                                    {" "}
                                    <SafeTimeAgo since={comment.created.as_ref()} lang />
                                </small>
                                <ContentView src={comment} />
                            </div>
                        </div>
                    </>
                }).render_into(writer)?;
            }
            RespNotificationInfo::UserFollow { user } => {
                (render::rsx! {
                    <div>
                        {
                            lang::TrElements::new(
                                lang.tr(&lang::notification_user_follow(lang::LangPlaceholder(0))),
                                |id, w| {
                                    match id {
                                        0 => render::rsx! {
                                            <UserLink lang user={Some(user)} />
                                        }.render_into(w),
                                        _ => unreachable!(),
                                    }
                                },
                            )
                        }
                    </div>
                })
                .render_into(writer)?;
            }
        }

        write!(writer, "</li>")
    }
}

pub struct SiteModlogEventItem<'a> {
    pub event: &'a RespSiteModlogEvent<'a>,
    pub lang: &'a crate::Translator,
}

impl render::Render for SiteModlogEventItem<'_> {
    fn render_into<W: std::fmt::Write + ?Sized>(self, writer: &mut W) -> std::fmt::Result {
        let lang = self.lang;
        let event = &self.event;

        write!(writer, "<li>")?;

        (render::rsx! {
            <>
                <SafeTimeAgo since={event.time.as_ref()} lang />
                {" - "}
            </>
        })
        .render_into(writer)?;

        match &event.details {
            RespSiteModlogEventDetails::DeletePost { author, community } => {
                lang::TrElements::new(
                    lang.tr(&lang::modlog_event_delete_post(
                        lang::LangPlaceholder(0),
                        lang::LangPlaceholder(1),
                    )),
                    |id, w| match id {
                        0 => render::rsx! {
                            <UserLink user={Some(author)} lang={lang} />
                        }
                        .render_into(w),
                        1 => render::rsx! {
                            <CommunityLink community />
                        }
                        .render_into(w),
                        _ => unreachable!(),
                    },
                )
                .render_into(writer)?;
            }
            RespSiteModlogEventDetails::DeleteComment { author, post } => {
                lang::TrElements::new(
                    lang.tr(&lang::modlog_event_delete_comment(
                        lang::LangPlaceholder(0),
                        lang::LangPlaceholder(1),
                    )),
                    |id, w| match id {
                        0 => render::rsx! {
                            <UserLink user={Some(author)} lang={lang} />
                        }
                        .render_into(w),
                        1 => render::rsx! {
                            <a href={format!("/posts/{}", post.id)}>{post.title.as_ref()}</a>
                        }
                        .render_into(w),
                        _ => unreachable!(),
                    },
                )
                .render_into(writer)?;
            }
            RespSiteModlogEventDetails::SuspendUser { user } => {
                (render::rsx! {
                    <>
                        {lang.tr(&lang::MODLOG_EVENT_SUSPEND_USER)}
                        {" "}
                        <UserLink user={Some(user)} lang={lang} />
                    </>
                })
                .render_into(writer)?;
            }
            RespSiteModlogEventDetails::UnsuspendUser { user } => {
                (render::rsx! {
                    <>
                        {lang.tr(&lang::MODLOG_EVENT_UNSUSPEND_USER)}
                        {" "}
                        <UserLink user={Some(user)} lang={lang} />
                    </>
                })
                .render_into(writer)?;
            }
        }

        write!(writer, "</li>")?;

        Ok(())
    }
}

pub struct PollView<'a> {
    pub poll: &'a RespPollInfo<'a>,
    pub action: String,
    pub lang: &'a crate::Translator,
}
impl render::Render for PollView<'_> {
    fn render_into<W: std::fmt::Write + ?Sized>(self, writer: &mut W) -> std::fmt::Result {
        let PollView { poll, action, lang } = &self;

        if poll.your_vote.is_some() || poll.is_closed {
            let full_width_votes = f64::from(if poll.multiple {
                poll.options.iter().map(|x| x.votes).max().unwrap_or(0)
            } else {
                poll.options.iter().map(|x| x.votes).sum()
            });

            (render::rsx! {
                <div>
                    <table class={"pollResults"}>
                        {
                            poll.options.iter().map(|option| {
                                let selected = poll.your_vote.as_ref().is_some_and(|your_vote| your_vote.options.iter().any(|x| x.id == option.id));
                                render::rsx! {
                                    <tr class={if selected { "selected" } else { "" }}>
                                        <td class={"count"}>
                                            <div class={"background"} style={format!("width: {}%", f64::from(option.votes) * 100.0 / full_width_votes)}>{""}</div>
                                            {option.votes}
                                        </td>
                                        <td>{option.name.as_ref()}</td>
                                    </tr>
                                }
                            }).collect::<Vec<_>>()
                        }
                    </table>
                </div>
            }).render_into(writer)
        } else {
            (render::rsx! {
                <div>
                    <form method={"post"} action={action}>
                        {
                            if poll.multiple {
                                poll.options.iter().map(|option| {
                                    render::rsx! {
                                        <div>
                                            <label>
                                                <input type={"checkbox"} name={option.id.to_string()} />{" "}
                                                {option.name.as_ref()}
                                            </label>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()
                            } else {
                                poll.options.iter().map(|option| {
                                    render::rsx! {
                                        <div>
                                            <label>
                                                <input type={"radio"} name={"choice"} value={option.id.to_string()} />{" "}
                                                {option.name.as_ref()}
                                            </label>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()
                            }
                        }
                        <input type={"submit"} value={lang.tr(&lang::POLL_SUBMIT)} />
                    </form>
                </div>
            }).render_into(writer)
        }
    }
}

pub trait IconExt {
    fn img<'a>(&self, alt: impl Into<Cow<'a, str>>) -> render::SimpleElement<'a, ()>;
    fn img_aria_hidden(&self) -> render::SimpleElement<'static, ()>;
}

impl IconExt for hitide_icons::Icon {
    fn img<'a>(&self, alt: impl Into<Cow<'a, str>>) -> render::SimpleElement<'a, ()> {
        render::rsx! {
            <img src={format!("/static/{}", self.path)} class={if self.dark_invert { "icon darkInvert" } else { "icon" }} alt={alt.into()} />
        }
    }

    fn img_aria_hidden(&self) -> render::SimpleElement<'static, ()> {
        render::rsx! {
            <img src={format!("/static/{}", self.path)} class={if self.dark_invert { "icon darkInvert" } else { "icon" }} aria-hidden={"true"} />
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use render::Render;

    #[test]
    fn site_stylesheet_href_uses_custom_css_or_bundled_default() {
        let mut base_data = crate::PageBaseData {
            login: None,
            site_name: "lotide".to_owned(),
            site_logo_url: None,
            site_css_url: None,
        };

        assert_eq!(site_stylesheet_href(&base_data), "/static/main.css");

        base_data.site_css_url = Some("/api/stable/instance/stylesheet".to_owned());

        assert_eq!(
            site_stylesheet_href(&base_data),
            "/api/stable/instance/stylesheet"
        );
    }

    fn minimal_post_with_href<'a>(href: Option<&'a str>) -> RespPostListPost<'a> {
        RespPostListPost {
            base: crate::resp_types::RespSomePostInfo {
                base: crate::resp_types::RespMinimalPostInfo {
                    id: 42,
                    title: "test post".into(),
                    remote_url: None,
                    sensitive: false,
                },
                href: href.map(Cow::Borrowed),
                author: Some(crate::resp_types::RespMinimalAuthorInfo {
                    id: 7,
                    username: "alice".into(),
                    local: false,
                    host: "remote.example".into(),
                    remote_url: None,
                    avatar: None,
                    is_bot: false,
                }),
                created: "2026-01-01T00:00:00+00:00".into(),
                community: crate::resp_types::RespMinimalCommunityInfo {
                    id: 3,
                    name: "test".into(),
                    local: false,
                    host: "remote.example".into(),
                    remote_url: None,
                    deleted: false,
                },
                sticky: false,
                federation_status: None,
            },
            replies_count_total: 0,
        }
    }

    #[test]
    fn federation_status_badge_and_line_class_render_delivery_state() {
        let mut html = String::new();

        FederationStatusBadge {
            status: Some(RespFederationStatus::Received),
        }
        .render_into(&mut html)
        .unwrap();

        assert!(html.contains("federationStatusReceived"));
        assert!(html.contains("federation: received"));
        assert_eq!(
            federation_status_line_class(Some(RespFederationStatus::Sent)),
            "federationStatusLine federationStatusLineSent"
        );
        assert_eq!(federation_status_line_class(None), "");
    }

    #[test]
    fn follow_federation_status_badge_explains_follow_lifecycle() {
        let follow = RespYourFollow {
            accepted: false,
            federation_status: Some(RespFederationStatus::Received),
        };
        let mut html = String::new();

        FollowFederationStatusBadge {
            your_follow: Some(&follow),
            latest_unfollow_status: None,
        }
        .render_into(&mut html)
        .unwrap();

        assert!(html.contains("follow request: received by remote"));
        assert!(html.contains("federationStatusReceived"));

        html.clear();
        FollowFederationStatusBadge {
            your_follow: None,
            latest_unfollow_status: Some(RespFederationStatus::Sent),
        }
        .render_into(&mut html)
        .unwrap();

        assert!(html.contains("unfollow: sent"));
        assert!(html.contains("federationStatusSent"));
    }

    #[test]
    fn notification_item_renders_user_follow() {
        let lang = crate::get_lang_for_headers(&Default::default());
        let notification = RespNotification {
            info: RespNotificationInfo::UserFollow {
                user: RespMinimalAuthorInfo {
                    id: 7,
                    username: "remote_alice".into(),
                    local: false,
                    host: "social.example".into(),
                    remote_url: Some("https://social.example/users/remote_alice".into()),
                    avatar: None,
                    is_bot: false,
                },
            },
            unseen: true,
        };
        let mut html = String::new();

        NotificationItem {
            notification: &notification,
            lang: &lang,
        }
        .render_into(&mut html)
        .unwrap();

        assert!(html.contains("remote_alice"));
        assert!(html.contains("followed you"));
        assert!(html.contains("notification-item unread"));
    }

    #[test]
    fn user_link_renders_avatar_when_available() {
        let lang = crate::get_lang_for_headers(&Default::default());
        let user = RespMinimalAuthorInfo {
            id: 7,
            username: "alice".into(),
            local: true,
            host: "lotide.example".into(),
            remote_url: None,
            avatar: Some(crate::resp_types::JustURL {
                url: "/api/stable/users/7/avatar/href".into(),
            }),
            is_bot: false,
        };
        let mut html = String::new();

        UserLink {
            lang: &lang,
            user: Some(&user),
        }
        .render_into(&mut html)
        .unwrap();

        assert!(html.contains("class=\"userLink\""));
        assert!(html.contains("class=\"userAvatarSmall\""));
        assert!(html.contains("/api/stable/users/7/avatar/href"));
        assert!(html.contains("alice"));
    }

    #[test]
    fn content_view_escapes_plain_text_from_remote_sources() {
        let content = HavingContentRef {
            content_html: None,
            content_text: Some("<script>alert(1)</script>"),
        };
        let mut html = String::new();

        ContentView { src: &content }
            .render_into(&mut html)
            .unwrap();

        assert!(html.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn content_view_preserves_backend_sanitized_html() {
        let content = HavingContentRef {
            content_html: Some("<p><strong>ok</strong></p>"),
            content_text: Some("fallback"),
        };
        let mut html = String::new();

        ContentView { src: &content }
            .render_into(&mut html)
            .unwrap();

        assert!(html.contains("<p><strong>ok</strong></p>"));
        assert!(!html.contains("fallback"));
    }

    #[test]
    fn post_item_content_neutralizes_unsafe_remote_href() {
        let lang = crate::get_lang_for_headers(&Default::default());
        let post = minimal_post_with_href(Some("javascript:alert(1)"));
        let mut html = String::new();

        PostItemContent {
            post: &post,
            in_community: false,
            no_user: false,
            lang: &lang,
        }
        .render_into(&mut html)
        .unwrap();

        assert!(!html.contains("href=\"javascript:alert(1)\""));
        assert!(html.contains("href=\"#\""));
    }

    #[test]
    fn post_item_content_keeps_safe_remote_href() {
        let lang = crate::get_lang_for_headers(&Default::default());
        let post = minimal_post_with_href(Some("https://example.com/article"));
        let mut html = String::new();

        PostItemContent {
            post: &post,
            in_community: false,
            no_user: false,
            lang: &lang,
        }
        .render_into(&mut html)
        .unwrap();

        assert!(html.contains("href=\"https://example.com/article\""));
        assert!(html.contains("example.com"));
    }
}
