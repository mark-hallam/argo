use crate::trust::TrustGrade;

/// Forward conversion from a [`TrustGrade`] to its numeric
/// discriminant for storage in `skill.min_trust_grade`.
/// Matches the enum's declared variant order: Builtin=0,
/// PublishedVerified=1, Tenant=2, PublishedCommunity=3,
/// LearnedUnreviewed=4.
pub(crate) fn trust_grade_as_u8(grade: TrustGrade) -> u8 {
    match grade {
        TrustGrade::Builtin => 0,
        TrustGrade::PublishedVerified => 1,
        TrustGrade::Tenant => 2,
        TrustGrade::PublishedCommunity => 3,
        TrustGrade::LearnedUnreviewed => 4,
    }
}

/// Reverse conversion. Returns `None` for unknown discriminants.
pub(crate) fn trust_grade_from_u8(byte: u8) -> Option<TrustGrade> {
    match byte {
        0 => Some(TrustGrade::Builtin),
        1 => Some(TrustGrade::PublishedVerified),
        2 => Some(TrustGrade::Tenant),
        3 => Some(TrustGrade::PublishedCommunity),
        4 => Some(TrustGrade::LearnedUnreviewed),
        _ => None,
    }
}
