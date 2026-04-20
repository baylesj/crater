//! Ranking strategies for track ordering.

use crate::error::{CoreError, Result};
use crate::tracks::StoredTrack;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ranking {
    /// likes/plays ratio — "hidden gem" signal. Filters out <5-play tracks.
    EngagementRatio,
    /// Sort by last_seen descending — prioritise recent uploads.
    Recency,
    /// Deterministic pseudo-shuffle keyed by current time. Serendipity mode.
    Shuffle,
    /// log(likes+1) / sqrt(plays+1) — balances engagement with obscurity.
    /// Preferred for digests: less fooled by 2-play / 2-like tracks.
    Score,
}

impl Ranking {
    pub fn as_str(&self) -> &'static str {
        match self {
            Ranking::EngagementRatio => "engagement_ratio",
            Ranking::Recency         => "recency",
            Ranking::Shuffle         => "shuffle",
            Ranking::Score           => "score",
        }
    }
}

impl std::str::FromStr for Ranking {
    type Err = CoreError;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "engagement_ratio" => Ok(Self::EngagementRatio),
            "recency"          => Ok(Self::Recency),
            "shuffle"          => Ok(Self::Shuffle),
            "score"            => Ok(Self::Score),
            other              => Err(CoreError::InvalidRanking(other.to_owned())),
        }
    }
}

/// Sort a Vec of tracks by the given ranking strategy, best first.
pub fn rank_tracks(mut tracks: Vec<StoredTrack>, ranking: &Ranking) -> Vec<StoredTrack> {
    match ranking {
        Ranking::EngagementRatio => {
            tracks.retain(|t| t.playback_count.unwrap_or(0) >= 5);
            tracks.sort_by(|a, b| {
                b.engagement_ratio()
                    .unwrap_or(0.0)
                    .partial_cmp(&a.engagement_ratio().unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        Ranking::Score => {
            tracks.sort_by(|a, b| {
                b.score()
                    .unwrap_or(0.0)
                    .partial_cmp(&a.score().unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        Ranking::Recency => {
            tracks.sort_by(|a, b| b.last_seen.cmp(&a.last_seen));
        }
        Ranking::Shuffle => {
            use std::collections::hash_map::DefaultHasher;
            use std::hash::{Hash, Hasher};
            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            tracks.sort_by_key(|t| {
                let mut h = DefaultHasher::new();
                t.id.hash(&mut h);
                seed.hash(&mut h);
                h.finish()
            });
        }
    }
    tracks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: i64, likes: i64, plays: i64) -> StoredTrack {
        StoredTrack {
            id,
            likes_count: Some(likes),
            playback_count: Some(plays),
            title: None, artist: None, artist_sc_id: None,
            permalink_url: String::new(),
            duration_ms: None, bpm: None, genre: None, tag_list: None,
            reposts_count: None, comment_count: None, created_at_sc: None,
            first_seen: chrono::Utc::now(),
            last_seen:  chrono::Utc::now(),
            raw_json: None, status: None, status_note: None,
        }
    }

    #[test]
    fn score_ranking_orders_correctly() {
        let tracks = vec![
            track(1, 2,  2),    // tiny but perfect ratio — still scores well
            track(2, 50, 200),  // solid engagement
            track(3, 5,  1000), // many plays, low engagement — should rank last
        ];
        let ranked = rank_tracks(tracks, &Ranking::Score);
        assert_eq!(ranked.last().unwrap().id, 3);
    }

    #[test]
    fn engagement_ratio_filters_low_play_tracks() {
        let tracks = vec![track(1, 2, 2), track(2, 10, 100)];
        let ranked = rank_tracks(tracks, &Ranking::EngagementRatio);
        // track 1 has < 5 plays, should be removed
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].id, 2);
    }
}
