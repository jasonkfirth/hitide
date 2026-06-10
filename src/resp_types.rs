use serde_derive::Deserialize;
use std::borrow::Cow;

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum RespFlagDetails<'a> {
    Post {
        #[serde(borrow)]
        post: RespPostListPost<'a>,
    },
}

#[derive(Deserialize, Debug)]
pub struct RespFlagInfo<'a> {
    pub id: i64,
    pub flagger: RespMinimalAuthorInfo<'a>,
    pub created_local: Cow<'a, str>,
    pub content: Option<JustContentText<'a>>,
    #[serde(borrow)]
    #[serde(flatten)]
    pub details: RespFlagDetails<'a>,
}

#[derive(Deserialize, Debug)]
pub struct RespMinimalAuthorInfo<'a> {
    pub id: i64,
    pub username: Cow<'a, str>,
    pub local: bool,
    pub host: Cow<'a, str>,
    pub remote_url: Option<Cow<'a, str>>,
    #[serde(default)]
    pub avatar: Option<JustURL<'a>>,
    pub is_bot: bool,
}

#[derive(Deserialize, Debug)]
pub struct RespMinimalPostInfo<'a> {
    pub id: i64,
    pub title: Cow<'a, str>,
    pub remote_url: Option<Cow<'a, str>>,
    pub sensitive: bool,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RespFederationStatus {
    Unsent,
    Sent,
    Received,
    Posted,
}

#[derive(Deserialize, Debug)]
pub struct RespSomePostInfo<'a> {
    #[serde(flatten)]
    pub base: RespMinimalPostInfo<'a>,
    pub href: Option<Cow<'a, str>>,
    #[serde(borrow)]
    pub author: Option<RespMinimalAuthorInfo<'a>>,
    pub created: Cow<'a, str>,
    #[serde(borrow)]
    pub community: RespMinimalCommunityInfo<'a>,
    pub sticky: bool,
    pub federation_status: Option<RespFederationStatus>,
}

impl<'a> AsRef<RespMinimalPostInfo<'a>> for RespSomePostInfo<'a> {
    fn as_ref(&self) -> &RespMinimalPostInfo<'a> {
        &self.base
    }
}

#[derive(Deserialize, Debug)]
pub struct RespPostListPost<'a> {
    #[serde(flatten, borrow)]
    pub base: RespSomePostInfo<'a>,
    pub replies_count_total: i64,
}

impl<'a> AsRef<RespSomePostInfo<'a>> for RespPostListPost<'a> {
    fn as_ref(&self) -> &RespSomePostInfo<'a> {
        &self.base
    }
}

#[derive(Deserialize, Debug)]
pub struct RespCommunityLastPostInfo<'a> {
    #[serde(flatten)]
    pub base: RespMinimalPostInfo<'a>,
    pub created: Cow<'a, str>,
}

impl<'a> AsRef<RespMinimalPostInfo<'a>> for RespCommunityLastPostInfo<'a> {
    fn as_ref(&self) -> &RespMinimalPostInfo<'a> {
        &self.base
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum RespThingInfo<'a> {
    #[serde(rename = "post")]
    Post(RespPostListPost<'a>),
    #[serde(rename = "comment")]
    #[serde(borrow)]
    Comment(RespThingComment<'a>),
}

#[derive(Deserialize, Debug)]
pub struct RespMinimalCommentInfo<'a> {
    pub id: i64,
    pub content_text: Option<Cow<'a, str>>,
    pub content_html: Option<Cow<'a, str>>,
    pub sensitive: bool,
    pub remote_url: Option<Cow<'a, str>>,
}

#[derive(Deserialize, Debug)]
pub struct RespThingComment<'a> {
    #[serde(flatten)]
    pub base: RespMinimalCommentInfo<'a>,

    pub created: Cow<'a, str>,
    #[serde(borrow)]
    pub post: RespMinimalPostInfo<'a>,
    pub federation_status: Option<RespFederationStatus>,
}

impl<'a> AsRef<RespMinimalCommentInfo<'a>> for RespThingComment<'a> {
    fn as_ref(&self) -> &RespMinimalCommentInfo<'a> {
        &self.base
    }
}

#[derive(Deserialize, Debug)]
pub struct JustURL<'a> {
    pub url: Cow<'a, str>,
}

#[derive(Deserialize, Debug)]
pub struct RespPostCommentInfo<'a> {
    #[serde(flatten)]
    pub base: RespMinimalCommentInfo<'a>,

    pub attachments: Vec<JustURL<'a>>,

    pub author: Option<RespMinimalAuthorInfo<'a>>,
    pub created: Cow<'a, str>,
    pub local: bool,
    pub your_vote: Option<RespYourVote>,
    pub federation_status: Option<RespFederationStatus>,
    pub replies: Option<RespList<'a, RespPostCommentInfo<'a>>>,
}

impl<'a> AsRef<RespMinimalCommentInfo<'a>> for RespPostCommentInfo<'a> {
    fn as_ref(&self) -> &RespMinimalCommentInfo<'a> {
        &self.base
    }
}

#[derive(Deserialize, Debug)]
pub struct RespCommentInfo<'a> {
    #[serde(flatten)]
    pub base: RespPostCommentInfo<'a>,

    pub parent: Option<JustID>,
    #[serde(borrow)]
    pub post: Option<RespMinimalPostInfo<'a>>,
}

impl<'a> AsRef<RespPostCommentInfo<'a>> for RespCommentInfo<'a> {
    fn as_ref(&self) -> &RespPostCommentInfo<'a> {
        &self.base
    }
}

#[derive(Deserialize, Debug)]
pub struct RespPostInfo<'a> {
    #[serde(flatten, borrow)]
    pub base: RespSomePostInfo<'a>,

    pub content_text: Option<Cow<'a, str>>,
    pub content_html: Option<Cow<'a, str>>,
    pub approved: bool,
    pub rejected: bool,
    pub score: i64,
    pub local: bool,
    pub your_vote: Option<RespYourVote>,
    pub poll: Option<RespPollInfo<'a>>,
}

impl<'a> AsRef<RespSomePostInfo<'a>> for RespPostInfo<'a> {
    fn as_ref(&self) -> &RespSomePostInfo<'a> {
        &self.base
    }
}

#[derive(Deserialize, Debug)]
pub struct RespPollInfo<'a> {
    pub multiple: bool,
    pub options: Vec<RespPollOption<'a>>,
    pub is_closed: bool,
    pub your_vote: Option<RespPollYourVote>,
}

#[derive(Deserialize, Debug)]
pub struct RespPollOption<'a> {
    pub id: i64,
    pub name: Cow<'a, str>,
    pub votes: u32,
}

#[derive(Deserialize, Debug)]
pub struct RespPollYourVote {
    pub options: Vec<JustID>,
}

#[derive(Deserialize, Debug)]
pub struct RespMinimalCommunityInfo<'a> {
    pub id: i64,
    pub name: Cow<'a, str>,
    pub local: bool,
    pub host: Cow<'a, str>,
    pub remote_url: Option<Cow<'a, str>>,
    pub deleted: bool,
}

#[derive(Deserialize, Debug)]
pub struct RespUserInfo<'a> {
    #[serde(flatten)]
    pub base: RespMinimalAuthorInfo<'a>,
    pub description: Content<'a>,
    pub suspended: Option<bool>,
    pub your_note: Option<JustContentText<'a>>,
}

impl<'a> AsRef<RespMinimalAuthorInfo<'a>> for RespUserInfo<'a> {
    fn as_ref(&self) -> &RespMinimalAuthorInfo<'a> {
        &self.base
    }
}

#[derive(Deserialize, Debug)]
pub struct RespLoginInfoUser {
    pub id: i64,
    pub is_site_admin: bool,
    pub has_unread_notifications: bool,
    pub has_pending_moderation_actions: bool,
}

#[derive(Deserialize, Debug)]
pub struct RespLoginInfo {
    pub user: RespLoginInfoUser,
    pub permissions: RespLoginPermissions,
}

#[derive(Deserialize, Debug)]
pub struct RespLoginPermissions {
    pub create_community: RespPermissionInfo,
    pub create_invitation: RespPermissionInfo,
}

#[derive(Deserialize, Debug)]
pub struct RespPermissionInfo {
    pub allowed: bool,
}

#[derive(Deserialize, Debug)]
pub struct Empty {}

#[derive(Deserialize, Debug)]
pub struct JustID {
    pub id: i64,
}

#[derive(Deserialize, Debug)]
pub struct JustStringID<'a> {
    pub id: &'a str,
}

#[derive(Deserialize, Debug)]
pub struct RespYourFollow {
    pub accepted: bool,
    pub federation_status: Option<RespFederationStatus>,
}

#[derive(Deserialize, Debug)]
pub struct RespYourVote {
    pub federation_status: Option<RespFederationStatus>,
}

#[derive(Deserialize, Debug)]
pub struct RespCommunityFeedsType<'a> {
    pub new: Cow<'a, str>,
}

#[derive(Deserialize, Debug)]
pub struct RespCommunityFeeds<'a> {
    pub atom: RespCommunityFeedsType<'a>,
}

#[derive(Deserialize, Debug)]
pub struct RespCommunityVisibilitySuppression {
    pub server: bool,
    pub user: bool,
}

#[derive(Deserialize, Debug)]
pub struct RespCommunityInfoMaybeYour<'a> {
    #[serde(flatten)]
    pub base: RespMinimalCommunityInfo<'a>,

    pub description: Content<'a>,
    pub feeds: RespCommunityFeeds<'a>,

    pub you_are_moderator: Option<bool>,
    pub your_follow: Option<RespYourFollow>,
    pub last_post: Option<RespCommunityLastPostInfo<'a>>,
    pub remote_post_count: Option<i64>,
    pub latest_unfollow_status: Option<RespFederationStatus>,
    pub visibility_suppression: Option<RespCommunityVisibilitySuppression>,
    pub pending_moderation_actions: Option<u32>,
}

impl<'a> AsRef<RespMinimalCommunityInfo<'a>> for RespCommunityInfoMaybeYour<'a> {
    fn as_ref(&self) -> &RespMinimalCommunityInfo<'a> {
        &self.base
    }
}

#[derive(Deserialize, Debug)]
pub struct RespCollectionTargetOwner<'a> {
    pub id: Option<i64>,
    pub remote_url: Option<Cow<'a, str>>,
}

#[derive(Deserialize, Debug)]
pub struct RespCollectionTargetPreviewItem<'a> {
    pub id: i64,
    pub ap_id: Cow<'a, str>,
    #[serde(rename = "type")]
    pub kind: Option<Cow<'a, str>>,
    pub name: Cow<'a, str>,
    pub url: Option<Cow<'a, str>>,
    pub attributed_to: Option<Cow<'a, str>>,
    pub content_html: Option<Cow<'a, str>>,
    pub summary_html: Option<Cow<'a, str>>,
    pub image_url: Option<Cow<'a, str>>,
    pub published: Option<Cow<'a, str>>,
}

#[derive(Deserialize, Debug)]
pub struct RespCollectionTargetInfo<'a> {
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: Cow<'a, str>,
    pub software: Option<Cow<'a, str>>,
    pub name: Cow<'a, str>,
    pub remote_url: Cow<'a, str>,
    pub owner: RespCollectionTargetOwner<'a>,
    pub followers: Option<Cow<'a, str>>,
    pub first_page: Option<Cow<'a, str>>,
    pub last_page: Option<Cow<'a, str>>,
    pub summary_html: Option<Cow<'a, str>>,
    pub total_items: Option<i64>,
    pub your_follow: Option<RespYourFollow>,
    pub latest_unfollow_status: Option<RespFederationStatus>,
    #[serde(default)]
    pub preview_items: Vec<RespCollectionTargetPreviewItem<'a>>,
}

#[derive(Deserialize, Debug)]
pub struct RespInstanceSoftwareInfo<'a> {
    pub name: Cow<'a, str>,
    pub version: Cow<'a, str>,
}

fn default_site_name<'a>() -> Cow<'a, str> {
    Cow::Borrowed("lotide")
}

fn default_cleanup_remote_post_retention_days() -> i32 {
    30
}

fn default_cleanup_preview_post_retention_hours() -> i32 {
    24
}

fn default_cleanup_notification_retention_days() -> i32 {
    90
}

fn default_cleanup_failed_inbox_task_payload_retention_days() -> i32 {
    30
}

#[derive(Deserialize, Debug)]
pub struct RespInstanceInfo<'a> {
    pub description: Content<'a>,
    pub software: RespInstanceSoftwareInfo<'a>,
    #[serde(default = "default_site_name")]
    pub site_name: Cow<'a, str>,
    #[serde(default)]
    pub site_logo: Option<JustURL<'a>>,
    #[serde(default)]
    pub site_css: Option<JustURL<'a>>,
    pub signup_allowed: bool,
    pub invitations_enabled: bool,
    pub community_creation_requirement: Option<Cow<'a, str>>,
    pub invitation_creation_requirement: Option<Cow<'a, str>>,
    #[serde(default)]
    pub cleanup_remote_posts_enabled: bool,
    #[serde(default = "default_cleanup_remote_post_retention_days")]
    pub cleanup_remote_post_retention_days: i32,
    #[serde(default)]
    pub cleanup_preview_posts_enabled: bool,
    #[serde(default = "default_cleanup_preview_post_retention_hours")]
    pub cleanup_preview_post_retention_hours: i32,
    #[serde(default)]
    pub cleanup_deleted_remote_communities_enabled: bool,
    #[serde(default)]
    pub cleanup_unfollowed_remote_communities_enabled: bool,
    #[serde(default)]
    pub cleanup_remote_interactions_enabled: bool,
    #[serde(default)]
    pub cleanup_notifications_enabled: bool,
    #[serde(default = "default_cleanup_notification_retention_days")]
    pub cleanup_notification_retention_days: i32,
    #[serde(default)]
    pub cleanup_failed_inbox_task_payloads_enabled: bool,
    #[serde(default = "default_cleanup_failed_inbox_task_payload_retention_days")]
    pub cleanup_failed_inbox_task_payload_retention_days: i32,

    pub web_push_vapid_key: Cow<'a, str>,
}

#[derive(Deserialize, Debug)]
pub struct RespAdminHostProfile {
    pub host: String,
    pub software: Option<String>,
    pub active: bool,
    pub last_checked: Option<String>,
    pub last_success: Option<String>,
    pub failed_checks: i32,
    pub latest_error: Option<String>,
    pub suppressed_reason: Option<String>,
    pub suppressed_at: Option<String>,
    pub interaction_probe_checked_at: Option<String>,
    pub interaction_probe_success_at: Option<String>,
    pub interaction_probe_latest_error: Option<String>,
    pub discovered_communities_total: i64,
    pub discovered_communities_active: i64,
    pub discovered_communities_with_posts: i64,
    pub communities_total: i64,
    pub followed_communities_total: i64,
    pub actor_profiles_total: i64,
    pub high_confidence_actor_profiles_total: i64,
    pub recent_events_total: i64,
    pub recent_failures_total: i64,
}

#[derive(Deserialize, Debug)]
pub struct RespAdminFederationSummary {
    pub discovery_servers_total: i64,
    pub discovery_servers_active: i64,
    pub discovery_servers_inactive: i64,
    pub discovery_servers_suppressed: i64,
    pub discovery_servers_probe_success: i64,
    pub discovered_communities_total: i64,
    pub discovered_communities_active: i64,
    pub discovered_communities_with_posts: i64,
    pub actor_target_profiles_total: i64,
    pub blocked_ap_ids_total: i64,
    pub server_suppressed_communities_total: i64,
    pub user_suppressed_communities_total: i64,
    pub federation_events_total: i64,
}

#[derive(Deserialize, Debug)]
pub struct RespAdminFederationServer {
    pub host: String,
    pub software: Option<String>,
    pub active: bool,
    pub last_checked: Option<String>,
    pub last_success: Option<String>,
    pub failed_checks: i32,
    pub latest_error: Option<String>,
    pub suppressed_reason: Option<String>,
    pub suppressed_at: Option<String>,
    pub interaction_probe_checked_at: Option<String>,
    pub interaction_probe_success_at: Option<String>,
    pub interaction_probe_latest_error: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct RespAdminFederationBlockedApId {
    pub ap_id: String,
}

#[derive(Deserialize, Debug)]
pub struct RespAdminFederationServerSuppressedCommunity {
    pub community_id: i64,
    pub community_name: String,
    pub community_ap_id: Option<String>,
    pub reason: String,
    pub updated_at: String,
}

#[derive(Deserialize, Debug)]
pub struct RespAdminFederationUserSuppressedCommunity {
    pub community_id: i64,
    pub community_name: String,
    pub community_ap_id: Option<String>,
    pub person_id: i64,
    pub username: String,
    pub person_ap_id: Option<String>,
    pub reason: String,
    pub updated_at: String,
}

#[derive(Deserialize, Debug)]
pub struct RespAdminFederationActorProfileFamily {
    pub family: String,
    pub target: String,
    pub actor_kind: String,
    pub count: i64,
    pub high_confidence_count: i64,
}

#[derive(Deserialize, Debug)]
pub struct RespAdminFederationActorProfile {
    pub actor_ap_id: String,
    pub target: String,
    pub family: String,
    pub actor_kind: String,
    pub source: String,
    pub confidence: i32,
    pub has_inbox: bool,
    pub has_outbox: bool,
    pub has_followers: bool,
    pub has_featured: bool,
    pub observed_object_types: Vec<String>,
    pub observed_activity_types: Vec<String>,
    pub updated_at: String,
}

#[derive(Deserialize, Debug)]
pub struct RespAdminFederationEvent {
    pub direction: String,
    pub action: String,
    pub status: String,
    pub host: Option<String>,
    pub actor_ap_id: Option<String>,
    pub object_ap_id: Option<String>,
    pub target_ap_id: Option<String>,
    pub activity_type: Option<String>,
    pub task_kind: Option<String>,
    pub error_class: Option<String>,
    pub error_text: Option<String>,
    pub created_at: String,
}

#[derive(Deserialize, Debug)]
pub struct RespAdminFederationReplayableTask {
    pub id: i64,
    pub kind: String,
    pub state: String,
    pub attempts: i32,
    pub max_attempts: i32,
    pub latest_error: Option<String>,
    pub created_at: String,
    pub attempted_at: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct RespAdminFederationHealth {
    pub summary: RespAdminFederationSummary,
    pub suppressed_servers: Vec<RespAdminFederationServer>,
    pub failing_servers: Vec<RespAdminFederationServer>,
    pub host_profiles: Vec<RespAdminHostProfile>,
    pub blocked_ap_ids: Vec<RespAdminFederationBlockedApId>,
    pub server_suppressed_communities: Vec<RespAdminFederationServerSuppressedCommunity>,
    pub user_suppressed_communities: Vec<RespAdminFederationUserSuppressedCommunity>,
    pub actor_profile_families: Vec<RespAdminFederationActorProfileFamily>,
    pub recent_actor_profiles: Vec<RespAdminFederationActorProfile>,
    pub recent_events: Vec<RespAdminFederationEvent>,
    pub replayable_failed_tasks: Vec<RespAdminFederationReplayableTask>,
}

#[derive(Deserialize, Debug)]
pub struct RespInvitationInfo<'a> {
    pub id: i32,
    pub key: Cow<'a, str>,
    pub created_by: RespMinimalAuthorInfo<'a>,
    pub created_at: Cow<'a, str>,
    pub used: bool,
}

#[derive(Deserialize, Debug)]
pub struct InvitationsCreateResponse<'a> {
    pub id: i32,
    pub key: Cow<'a, str>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum RespCommunityModlogEventDetails<'a> {
    RejectPost { post: RespMinimalPostInfo<'a> },
    ApprovePost { post: RespMinimalPostInfo<'a> },
}

#[derive(Deserialize, Debug)]
pub struct RespCommunityModlogEvent<'a> {
    pub time: Cow<'a, str>,
    #[serde(flatten)]
    pub details: RespCommunityModlogEventDetails<'a>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum RespSiteModlogEventDetails<'a> {
    DeletePost {
        author: RespMinimalAuthorInfo<'a>,
        community: RespMinimalCommunityInfo<'a>,
    },
    DeleteComment {
        author: RespMinimalAuthorInfo<'a>,
        post: RespMinimalPostInfo<'a>,
    },
    SuspendUser {
        user: RespMinimalAuthorInfo<'a>,
    },
    UnsuspendUser {
        user: RespMinimalAuthorInfo<'a>,
    },
}

#[derive(Deserialize, Debug)]
pub struct RespSiteModlogEvent<'a> {
    pub time: Cow<'a, str>,
    #[serde(flatten)]
    pub details: RespSiteModlogEventDetails<'a>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum RespNotificationInfo<'a> {
    PostReply {
        reply: RespPostCommentInfo<'a>,
        #[serde(borrow)]
        post: RespPostListPost<'a>,
    },
    PostMention {
        #[serde(borrow)]
        post: RespPostListPost<'a>,
    },
    CommentReply {
        reply: RespPostCommentInfo<'a>,
        comment: RespPostCommentInfo<'a>,
        #[serde(borrow)]
        post: RespPostListPost<'a>,
    },
    CommentMention {
        comment: RespPostCommentInfo<'a>,
        #[serde(borrow)]
        post: RespPostListPost<'a>,
    },
    UserFollow {
        user: RespMinimalAuthorInfo<'a>,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize, Debug)]
pub struct RespNotification<'a> {
    #[serde(flatten)]
    #[serde(borrow)]
    pub info: RespNotificationInfo<'a>,

    pub unseen: bool,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Content<'a> {
    pub content_text: Option<Cow<'a, str>>,
    pub content_markdown: Option<Cow<'a, str>>,
    pub content_html: Option<Cow<'a, str>>,
}

#[derive(Deserialize, Debug)]
pub struct JustUser<'a> {
    pub user: RespMinimalAuthorInfo<'a>,
}

#[derive(Deserialize, Debug)]
pub struct RespLikeInfo<'a> {
    pub user: RespMinimalAuthorInfo<'a>,
    pub federation_status: Option<RespFederationStatus>,
}

#[derive(Deserialize, Debug)]
pub struct JustContentText<'a> {
    pub content_text: Cow<'a, str>,
}

#[derive(Deserialize, Debug)]
pub struct JustContentHTML<'a> {
    pub content_html: Cow<'a, str>,
}

#[derive(Deserialize, Debug)]
pub struct RespList<'a, T: std::fmt::Debug + 'a> {
    pub items: Vec<T>,
    pub next_page: Option<Cow<'a, str>>,
    #[serde(default)]
    pub total_count: Option<i64>,
}
