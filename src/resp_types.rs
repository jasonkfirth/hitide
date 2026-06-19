use serde_derive::Deserialize;
use std::borrow::Cow;

fn default_site_name<'a>() -> Cow<'a, str> {
    Cow::Borrowed("lotide")
}

fn default_software<'a>() -> RespInstanceSoftwareInfo<'a> {
    RespInstanceSoftwareInfo {
        name: Cow::Borrowed("lotide"),
        version: Cow::Borrowed("unknown"),
    }
}

fn default_true() -> bool {
    true
}

fn default_remote_post_retention_days() -> i32 {
    90
}

fn default_preview_post_retention_hours() -> i32 {
    2
}

fn default_notification_retention_days() -> i32 {
    365
}

fn default_failed_inbox_task_payload_retention_days() -> i32 {
    7
}

fn default_completed_task_retention_days() -> i32 {
    3
}

fn default_failed_task_retention_days() -> i32 {
    14
}

fn default_failed_inbox_task_payload_compaction_hours() -> i32 {
    1
}

fn default_discovery_enqueue_limit() -> i32 {
    100
}

fn default_discovery_refresh_interval_hours() -> i32 {
    6
}

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
pub struct RespPrivateMessageInfo<'a> {
    pub id: i64,
    #[serde(borrow)]
    pub author: RespMinimalAuthorInfo<'a>,
    #[serde(borrow)]
    pub recipient: RespMinimalAuthorInfo<'a>,
    pub created: Cow<'a, str>,
    pub local: bool,
    pub remote_url: Option<Cow<'a, str>>,
    pub content_text: Option<Cow<'a, str>>,
    pub content_markdown: Option<Cow<'a, str>>,
    pub content_html: Option<Cow<'a, str>>,
    pub in_reply_to: Option<i64>,
    pub federation_status: Option<RespFederationStatus>,
    pub sensitive: bool,
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
    #[serde(default)]
    pub your_follow: Option<RespYourFollow>,
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
pub struct RespCommunityDiscoveryInfo<'a> {
    #[serde(default)]
    pub host: Option<Cow<'a, str>>,
    #[serde(default)]
    pub last_seen: Option<Cow<'a, str>>,
    #[serde(default)]
    pub server_last_success: Option<Cow<'a, str>>,
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
    #[serde(default)]
    pub discovery: Option<RespCommunityDiscoveryInfo<'a>>,
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
    #[serde(default)]
    pub your_vote: Option<RespYourVote>,
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
    #[serde(default = "default_true")]
    pub preview_item_likes_supported: bool,
    #[serde(default)]
    pub preview_items: Vec<RespCollectionTargetPreviewItem<'a>>,
}

#[derive(Deserialize, Debug)]
pub struct RespCollectionTargetItemCollection<'a> {
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: Cow<'a, str>,
    pub software: Option<Cow<'a, str>>,
    pub name: Cow<'a, str>,
    pub remote_url: Cow<'a, str>,
    pub owner: RespCollectionTargetOwner<'a>,
    #[serde(default = "default_true")]
    pub preview_item_likes_supported: bool,
    #[serde(default)]
    pub preview_item_replies_supported: bool,
    #[serde(default)]
    pub can_reply: bool,
}

#[derive(Deserialize, Debug)]
pub struct RespCollectionTargetItemComment<'a> {
    pub id: i64,
    pub remote_url: Option<Cow<'a, str>>,
    pub content_text: Option<Cow<'a, str>>,
    pub content_markdown: Option<Cow<'a, str>>,
    pub content_html: Option<Cow<'a, str>>,
    pub created: Cow<'a, str>,
    pub local: bool,
    pub author: Option<RespMinimalAuthorInfo<'a>>,
    pub sensitive: bool,
    pub federation_status: Option<RespFederationStatus>,
}

#[derive(Deserialize, Debug)]
pub struct RespCollectionTargetItemInfo<'a> {
    pub collection: RespCollectionTargetItemCollection<'a>,
    pub item: RespCollectionTargetPreviewItem<'a>,
    #[serde(default)]
    pub comments: Vec<RespCollectionTargetItemComment<'a>>,
}

#[derive(Deserialize, Debug)]
pub struct RespCollectionTargetListItem<'a> {
    pub id: i64,
    #[serde(rename = "type")]
    pub kind: Cow<'a, str>,
    pub software: Cow<'a, str>,
    pub name: Cow<'a, str>,
    pub remote_url: Cow<'a, str>,
    pub owner: RespCollectionTargetOwner<'a>,
    pub total_items: Option<i64>,
    pub preview_item_count: i64,
    pub latest_preview_item: Option<Cow<'a, str>>,
    pub latest_preview_published: Option<Cow<'a, str>>,
    pub latest_preview_url: Option<Cow<'a, str>>,
    pub summary_excerpt: Option<Cow<'a, str>>,
    pub your_follow: Option<RespYourFollow>,
    pub latest_unfollow_status: Option<RespFederationStatus>,
}

#[derive(Deserialize, Debug)]
pub struct RespCollectionTargetSoftwareCount<'a> {
    pub software: Cow<'a, str>,
    pub count: i64,
}

#[derive(Deserialize, Debug)]
pub struct RespCollectionTargetList<'a> {
    pub items: Vec<RespCollectionTargetListItem<'a>>,
    pub next_page: Option<Cow<'a, str>>,
    pub total_count: i64,
    pub scope_total_count: i64,
    pub software_counts: Vec<RespCollectionTargetSoftwareCount<'a>>,
}

#[derive(Deserialize, Debug)]
pub struct RespInstanceSoftwareInfo<'a> {
    pub name: Cow<'a, str>,
    pub version: Cow<'a, str>,
}

#[derive(Deserialize, Debug)]
pub struct RespInstanceInfo<'a> {
    #[serde(default)]
    pub description: Content<'a>,
    #[serde(default = "default_software")]
    pub software: RespInstanceSoftwareInfo<'a>,
    #[serde(default = "default_site_name")]
    pub site_name: Cow<'a, str>,
    #[serde(default)]
    pub site_logo: Option<JustURL<'a>>,
    #[serde(default)]
    pub site_css: Option<JustURL<'a>>,
    #[serde(default)]
    pub signup_allowed: bool,
    #[serde(default)]
    pub invitations_enabled: bool,
    #[serde(default)]
    pub community_creation_requirement: Option<Cow<'a, str>>,
    #[serde(default)]
    pub invitation_creation_requirement: Option<Cow<'a, str>>,
    #[serde(default)]
    pub cleanup_remote_posts_enabled: bool,
    #[serde(default = "default_remote_post_retention_days")]
    pub cleanup_remote_post_retention_days: i32,
    #[serde(default)]
    pub cleanup_preview_posts_enabled: bool,
    #[serde(default = "default_preview_post_retention_hours")]
    pub cleanup_preview_post_retention_hours: i32,
    #[serde(default)]
    pub cleanup_deleted_remote_communities_enabled: bool,
    #[serde(default)]
    pub cleanup_unfollowed_remote_communities_enabled: bool,
    #[serde(default)]
    pub cleanup_remote_interactions_enabled: bool,
    #[serde(default)]
    pub cleanup_notifications_enabled: bool,
    #[serde(default = "default_notification_retention_days")]
    pub cleanup_notification_retention_days: i32,
    #[serde(default)]
    pub cleanup_failed_inbox_task_payloads_enabled: bool,
    #[serde(default = "default_failed_inbox_task_payload_retention_days")]
    pub cleanup_failed_inbox_task_payload_retention_days: i32,
    #[serde(default = "default_completed_task_retention_days")]
    pub cleanup_completed_task_retention_days: i32,
    #[serde(default = "default_failed_task_retention_days")]
    pub cleanup_failed_task_retention_days: i32,
    #[serde(default = "default_failed_inbox_task_payload_compaction_hours")]
    pub cleanup_failed_inbox_task_payload_compaction_hours: i32,
    #[serde(default = "default_discovery_enqueue_limit")]
    pub discovery_enqueue_limit: i32,
    #[serde(default = "default_discovery_refresh_interval_hours")]
    pub discovery_refresh_interval_hours: i32,

    #[serde(default)]
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
    #[serde(default)]
    pub failure_category: Option<String>,
    pub discovered_communities_total: i64,
    pub discovered_communities_active: i64,
    pub discovered_communities_with_posts: i64,
    #[serde(default)]
    pub useful_community_source: bool,
    pub communities_total: i64,
    pub followed_communities_total: i64,
    pub actor_profiles_total: i64,
    pub high_confidence_actor_profiles_total: i64,
    pub recent_events_total: i64,
    pub recent_failures_total: i64,
    #[serde(default)]
    pub newest_community_seen: Option<String>,
    #[serde(default)]
    pub catalog_status: Option<String>,
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
    #[serde(default)]
    pub discovered_communities_visible: i64,
    #[serde(default)]
    pub discovery_servers_useful_sources: i64,
    #[serde(default)]
    pub discovery_servers_known_only: i64,
    #[serde(default)]
    pub discovery_servers_due: i64,
    pub actor_target_profiles_total: i64,
    pub blocked_ap_ids_total: i64,
    pub server_suppressed_communities_total: i64,
    pub user_suppressed_communities_total: i64,
    pub federation_events_total: i64,
    #[serde(default)]
    pub task_pending_total: i64,
    #[serde(default)]
    pub task_running_total: i64,
    #[serde(default)]
    pub task_failed_total: i64,
    #[serde(default)]
    pub task_completed_total: i64,
    #[serde(default)]
    pub task_oldest_pending: Option<String>,
    #[serde(default)]
    pub task_table_bytes: i64,
    #[serde(default)]
    pub task_pending_outbound: i64,
    #[serde(default)]
    pub task_pending_inbox: i64,
    #[serde(default)]
    pub task_pending_discovery: i64,
    #[serde(default)]
    pub task_pending_preview: i64,
    #[serde(default)]
    pub task_pending_readback: i64,
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
    #[serde(default)]
    pub failure_category: Option<String>,
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
pub struct RespAdminFederationFollowedCommunityHealth {
    pub community_id: i64,
    pub community_name: String,
    pub community_ap_id: Option<String>,
    pub host: String,
    pub software: Option<String>,
    pub host_active: Option<bool>,
    pub host_failed_checks: Option<i32>,
    pub latest_error: Option<String>,
    pub suppressed_reason: Option<String>,
    pub last_success: Option<String>,
    pub local_followers: i64,
    pub visible_posts: i64,
    pub last_post: Option<String>,
    pub remote_post_count: i64,
    pub catalog_last_seen: Option<String>,
    pub health_status: String,
}

#[derive(Deserialize, Debug)]
pub struct RespAdminFederationHealth {
    pub summary: RespAdminFederationSummary,
    pub suppressed_servers: Vec<RespAdminFederationServer>,
    pub failing_servers: Vec<RespAdminFederationServer>,
    pub host_profiles: Vec<RespAdminHostProfile>,
    #[serde(default)]
    pub followed_community_health: Vec<RespAdminFederationFollowedCommunityHealth>,
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
    PrivateMessage {
        #[serde(borrow)]
        message: RespPrivateMessageInfo<'a>,
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

#[derive(Deserialize, Debug, Clone, Default)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_info_accepts_missing_newer_settings() {
        let info: RespInstanceInfo<'_> = serde_json::from_str("{}").unwrap();

        assert_eq!(info.site_name.as_ref(), "lotide");
        assert_eq!(info.software.name.as_ref(), "lotide");
        assert_eq!(info.software.version.as_ref(), "unknown");
        assert!(info.site_logo.is_none());
        assert!(info.site_css.is_none());
        assert!(!info.cleanup_remote_posts_enabled);
        assert_eq!(info.cleanup_remote_post_retention_days, 90);
        assert_eq!(info.cleanup_preview_post_retention_hours, 2);
        assert_eq!(info.cleanup_notification_retention_days, 365);
        assert_eq!(info.cleanup_failed_inbox_task_payload_retention_days, 7);
        assert_eq!(info.cleanup_completed_task_retention_days, 3);
        assert_eq!(info.cleanup_failed_task_retention_days, 14);
        assert_eq!(info.cleanup_failed_inbox_task_payload_compaction_hours, 1);
    }

    #[test]
    fn federation_summary_accepts_missing_task_metrics() {
        let summary: RespAdminFederationSummary = serde_json::from_str(
            r#"{
                "discovery_servers_total": 0,
                "discovery_servers_active": 0,
                "discovery_servers_inactive": 0,
                "discovery_servers_suppressed": 0,
                "discovery_servers_probe_success": 0,
                "discovered_communities_total": 0,
                "discovered_communities_active": 0,
                "discovered_communities_with_posts": 0,
                "actor_target_profiles_total": 0,
                "blocked_ap_ids_total": 0,
                "server_suppressed_communities_total": 0,
                "user_suppressed_communities_total": 0,
                "federation_events_total": 0
            }"#,
        )
        .unwrap();

        assert_eq!(summary.task_pending_total, 0);
        assert_eq!(summary.task_running_total, 0);
        assert_eq!(summary.task_failed_total, 0);
        assert_eq!(summary.task_completed_total, 0);
        assert!(summary.task_oldest_pending.is_none());
        assert_eq!(summary.task_table_bytes, 0);
        assert_eq!(summary.task_pending_outbound, 0);
        assert_eq!(summary.task_pending_inbox, 0);
        assert_eq!(summary.task_pending_discovery, 0);
        assert_eq!(summary.task_pending_preview, 0);
        assert_eq!(summary.task_pending_readback, 0);
    }

    #[test]
    fn federation_health_accepts_missing_followed_community_health() {
        let health: RespAdminFederationHealth = serde_json::from_str(
            r#"{
                "summary": {
                    "discovery_servers_total": 0,
                    "discovery_servers_active": 0,
                    "discovery_servers_inactive": 0,
                    "discovery_servers_suppressed": 0,
                    "discovery_servers_probe_success": 0,
                    "discovered_communities_total": 0,
                    "discovered_communities_active": 0,
                    "discovered_communities_with_posts": 0,
                    "actor_target_profiles_total": 0,
                    "blocked_ap_ids_total": 0,
                    "server_suppressed_communities_total": 0,
                    "user_suppressed_communities_total": 0,
                    "federation_events_total": 0
                },
                "suppressed_servers": [],
                "failing_servers": [],
                "host_profiles": [],
                "blocked_ap_ids": [],
                "server_suppressed_communities": [],
                "user_suppressed_communities": [],
                "actor_profile_families": [],
                "recent_actor_profiles": [],
                "recent_events": [],
                "replayable_failed_tasks": []
            }"#,
        )
        .unwrap();

        assert!(health.followed_community_health.is_empty());
    }

    #[test]
    fn instance_info_keeps_theme_fields_when_present() {
        let info: RespInstanceInfo<'_> = serde_json::from_str(
            r#"{
                "site_name": "Example Lotide",
                "site_logo": {"url": "/api/stable/instance/logo"},
                "site_css": {"url": "/api/stable/instance/stylesheet"}
            }"#,
        )
        .unwrap();

        assert_eq!(info.site_name.as_ref(), "Example Lotide");
        assert_eq!(
            info.site_logo.unwrap().url.as_ref(),
            "/api/stable/instance/logo"
        );
        assert_eq!(
            info.site_css.unwrap().url.as_ref(),
            "/api/stable/instance/stylesheet"
        );
    }

    #[test]
    fn collection_target_list_accepts_source_discovery_contract() {
        let list: RespCollectionTargetList<'_> = serde_json::from_str(
            r#"{
                "items": [{
                    "id": 12,
                    "type": "actor_feed",
                    "software": "castopod",
                    "name": "The Show",
                    "remote_url": "https://podcasts.example/@show",
                    "owner": {
                        "id": null,
                        "remote_url": "https://podcasts.example/@show"
                    },
                    "total_items": 42,
                    "preview_item_count": 3,
                    "latest_preview_item": "Episode 1",
                    "latest_preview_published": "2026-06-18T12:00:00Z",
                    "latest_preview_url": "https://podcasts.example/episodes/1",
                    "summary_excerpt": "A compact source summary",
                    "your_follow": {
                        "accepted": false,
                        "federation_status": "sent"
                    },
                    "latest_unfollow_status": null
                }],
                "next_page": null,
                "total_count": 1,
                "scope_total_count": 1,
                "software_counts": [{
                    "software": "castopod",
                    "count": 1
                }]
            }"#,
        )
        .unwrap();

        assert_eq!(list.items[0].software.as_ref(), "castopod");
        assert_eq!(
            list.items[0].summary_excerpt.as_deref(),
            Some("A compact source summary")
        );
        assert_eq!(
            list.items[0]
                .your_follow
                .as_ref()
                .unwrap()
                .federation_status,
            Some(RespFederationStatus::Sent)
        );
        assert_eq!(list.software_counts[0].software.as_ref(), "castopod");
    }

    #[test]
    fn collection_target_info_defaults_preview_likes_to_supported() {
        let target: RespCollectionTargetInfo<'_> = serde_json::from_str(
            r#"{
                "id": 12,
                "type": "actor_feed",
                "software": "postmarks",
                "name": "Bookmarks",
                "remote_url": "https://bookmarks.example/u/links",
                "owner": {
                    "id": null,
                    "remote_url": "https://bookmarks.example/u/links"
                },
                "followers": null,
                "first_page": "https://bookmarks.example/u/links/outbox?page=1",
                "last_page": null,
                "summary_html": null,
                "total_items": 10,
                "your_follow": null,
                "latest_unfollow_status": null,
                "preview_items": []
            }"#,
        )
        .unwrap();

        assert!(target.preview_item_likes_supported);

        let target: RespCollectionTargetInfo<'_> = serde_json::from_str(
            r#"{
                "id": 12,
                "type": "actor_feed",
                "software": "postmarks",
                "name": "Bookmarks",
                "remote_url": "https://bookmarks.example/u/links",
                "owner": {
                    "id": null,
                    "remote_url": "https://bookmarks.example/u/links"
                },
                "followers": null,
                "first_page": "https://bookmarks.example/u/links/outbox?page=1",
                "last_page": null,
                "summary_html": null,
                "total_items": 10,
                "your_follow": null,
                "latest_unfollow_status": null,
                "preview_item_likes_supported": false,
                "preview_items": []
            }"#,
        )
        .unwrap();

        assert!(!target.preview_item_likes_supported);
    }

    #[test]
    fn collection_target_item_info_accepts_native_reader_contract() {
        let item: RespCollectionTargetItemInfo<'_> = serde_json::from_str(
            r#"{
                "collection": {
                    "id": 12,
                    "type": "actor_feed",
                    "software": "wordpress",
                    "name": "A Blog",
                    "remote_url": "https://blog.example/ap/actor",
                    "owner": {
                        "id": null,
                        "remote_url": "https://blog.example/ap/actor"
                    },
                    "preview_item_likes_supported": true,
                    "preview_item_replies_supported": true,
                    "can_reply": true
                },
                "item": {
                    "id": 44,
                    "ap_id": "https://blog.example/posts/1",
                    "type": "Article",
                    "name": "Readable source item",
                    "url": "https://blog.example/readable-source-item",
                    "attributed_to": "https://blog.example/ap/actor",
                    "content_html": "<p>Cached body.</p>",
                    "summary_html": null,
                    "image_url": "https://blog.example/cover.jpg",
                    "published": "2026-06-19T12:00:00Z",
                    "your_vote": null
                },
                "comments": [{
                    "id": 6,
                    "remote_url": "https://lotide.example/apub/collection_targets/12/items/44/comments/6",
                    "content_text": null,
                    "content_markdown": "Good post.",
                    "content_html": "<p>Good post.</p>",
                    "created": "2026-06-19T12:30:00Z",
                    "local": true,
                    "author": {
                        "id": 1,
                        "username": "alice",
                        "local": true,
                        "host": "lotide.example",
                        "remote_url": "https://lotide.example/apub/users/1",
                        "avatar": null,
                        "is_bot": false
                    },
                    "sensitive": false,
                    "federation_status": "received"
                }]
            }"#,
        )
        .unwrap();

        assert_eq!(item.collection.name.as_ref(), "A Blog");
        assert!(item.collection.preview_item_replies_supported);
        assert!(item.collection.can_reply);
        assert_eq!(item.item.name.as_ref(), "Readable source item");
        assert_eq!(
            item.item.content_html.as_deref(),
            Some("<p>Cached body.</p>")
        );
        assert_eq!(
            item.comments[0].federation_status,
            Some(RespFederationStatus::Received)
        );
    }

    #[test]
    fn private_message_info_accepts_federation_status_contract() {
        let message: RespPrivateMessageInfo<'_> = serde_json::from_str(
            r#"{
                "id": 33,
                "author": {
                    "id": 1,
                    "username": "me",
                    "local": true,
                    "host": "lotide.example",
                    "remote_url": "https://lotide.example/apub/users/1",
                    "is_bot": false
                },
                "recipient": {
                    "id": 2,
                    "username": "remote",
                    "local": false,
                    "host": "remote.example",
                    "remote_url": "https://remote.example/users/remote",
                    "is_bot": false
                },
                "created": "2026-06-18T12:00:00Z",
                "local": true,
                "remote_url": "https://lotide.example/apub/private_messages/33",
                "content_text": "hello",
                "content_markdown": null,
                "content_html": "<p>hello</p>",
                "in_reply_to": null,
                "federation_status": "received",
                "sensitive": false
            }"#,
        )
        .unwrap();

        assert_eq!(
            message.federation_status,
            Some(RespFederationStatus::Received)
        );
        assert_eq!(message.content_html.as_deref(), Some("<p>hello</p>"));
    }
}

#[derive(Deserialize, Debug)]
pub struct RespList<'a, T: std::fmt::Debug + 'a> {
    pub items: Vec<T>,
    pub next_page: Option<Cow<'a, str>>,
    #[serde(default)]
    pub total_count: Option<i64>,
}
