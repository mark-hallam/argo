//! Worker v3 marketplace action-type enum (#728 legacy lock).
//!
//! Kept distinct from the OS-generic action_type strings used by
//! the manifest (#729) — verticals declare their action vocabulary
//! freely, but the SKYCOT safety boundary lock test in
//! `worker_actions_tests.rs` still pins which marketplace
//! actions require approval. Lives in its own file so
//! `worker_actions.rs` (which holds the schema + read-side CLI)
//! stays under the per-file 500-LOC budget.
//!
//! The enum is `dead_code` allowed because nothing in the live
//! call graph references it after #729's reframe; only the
//! safety boundary test consumes it.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionType {
    // Autonomous-safe (no approval required):
    CreateListingDraft,
    VerifyPrice,
    RequestPhotos,
    // Approval-gated:
    PostListing,
    BoostListing,
    ReplyToBuyer,
    FollowUpBuyer,
    MarkItemSold,
    UpdatePrice,
    RelistItem,
}

#[allow(dead_code)]
impl ActionType {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            ActionType::CreateListingDraft => "create_listing_draft",
            ActionType::VerifyPrice => "verify_price",
            ActionType::RequestPhotos => "request_photos",
            ActionType::PostListing => "post_listing",
            ActionType::BoostListing => "boost_listing",
            ActionType::ReplyToBuyer => "reply_to_buyer",
            ActionType::FollowUpBuyer => "follow_up_buyer",
            ActionType::MarkItemSold => "mark_item_sold",
            ActionType::UpdatePrice => "update_price",
            ActionType::RelistItem => "relist_item",
        }
    }

    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "create_listing_draft" => Some(Self::CreateListingDraft),
            "verify_price" => Some(Self::VerifyPrice),
            "request_photos" => Some(Self::RequestPhotos),
            "post_listing" => Some(Self::PostListing),
            "boost_listing" => Some(Self::BoostListing),
            "reply_to_buyer" => Some(Self::ReplyToBuyer),
            "follow_up_buyer" => Some(Self::FollowUpBuyer),
            "mark_item_sold" => Some(Self::MarkItemSold),
            "update_price" => Some(Self::UpdatePrice),
            "relist_item" => Some(Self::RelistItem),
            _ => None,
        }
    }

    /// Per the SKYCOT safety boundary: approval-gated action
    /// types may NEVER autonomously transition past `pending`.
    /// They require an explicit operator approval (recorded as
    /// a state-row transition `pending → approved`) before
    /// `approved → executing` can fire.
    pub(crate) fn requires_approval(&self) -> bool {
        matches!(
            self,
            Self::PostListing
                | Self::BoostListing
                | Self::ReplyToBuyer
                | Self::FollowUpBuyer
                | Self::MarkItemSold
                | Self::UpdatePrice
                | Self::RelistItem
        )
    }
}
