//! Wire types for the Telegram Bot API.
//!
//! Telegram's actual payload nests `chat: {id: ...}` and `from: {id: ...}`.
//! The public types in this module flatten those into `chat_id` /
//! `from_user_id` via custom (de)serialization, so callers don't have to
//! reach through wrapper objects. Bytes-in / bytes-out round-trips through
//! `serde_json` for the smoke test.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A message Telegram has accepted (or delivered to a webhook).
///
/// Mirrors the load-bearing fields of Telegram's `Message` object. Extra
/// fields on the wire are ignored.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelegramMessage {
    /// Telegram-assigned message id within the chat.
    pub message_id: i64,
    /// Flattened from Telegram's `chat: { id: ... }` envelope.
    #[serde(
        rename = "chat",
        deserialize_with = "deserialize_chat_id",
        serialize_with = "serialize_chat_id"
    )]
    pub chat_id: i64,
    /// Unix epoch seconds when Telegram received the message.
    pub date: i64,
}

/// Inbound webhook delivery from Telegram.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramWebhookUpdate {
    /// Monotonic update id; treat as opaque.
    pub update_id: i64,
    /// Present for `message` updates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<TelegramMessage>,
    /// Present for `callback_query` updates (inline keyboard taps).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callback_query: Option<TelegramCallbackQuery>,
}

/// A user's tap on an inline-keyboard button.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TelegramCallbackQuery {
    /// Callback id — pass back to `answerCallbackQuery`.
    pub id: String,
    /// Flattened from Telegram's `from: { id: ... }` envelope.
    #[serde(
        rename = "from",
        deserialize_with = "deserialize_user_id",
        serialize_with = "serialize_user_id"
    )]
    pub from_user_id: i64,
    /// Operator-defined payload string set when the button was rendered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

fn deserialize_chat_id<'de, D: Deserializer<'de>>(de: D) -> Result<i64, D::Error> {
    #[derive(Deserialize)]
    struct Chat {
        id: i64,
    }
    Chat::deserialize(de).map(|c| c.id)
}

fn serialize_chat_id<S: Serializer>(id: &i64, ser: S) -> Result<S::Ok, S::Error> {
    #[derive(Serialize)]
    struct Chat {
        id: i64,
    }
    Chat { id: *id }.serialize(ser)
}

fn deserialize_user_id<'de, D: Deserializer<'de>>(de: D) -> Result<i64, D::Error> {
    #[derive(Deserialize)]
    struct User {
        id: i64,
    }
    User::deserialize(de).map(|u| u.id)
}

fn serialize_user_id<S: Serializer>(id: &i64, ser: S) -> Result<S::Ok, S::Error> {
    #[derive(Serialize)]
    struct User {
        id: i64,
    }
    User { id: *id }.serialize(ser)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_decodes_from_telegram_wire_shape() {
        let wire = r#"{"message_id":123,"chat":{"id":456,"type":"private"},"date":1700000000,"text":"hi"}"#;
        let msg: TelegramMessage = serde_json::from_str(wire).unwrap();
        assert_eq!(msg.message_id, 123);
        assert_eq!(msg.chat_id, 456);
        assert_eq!(msg.date, 1700000000);
    }

    #[test]
    fn update_with_callback_query_decodes() {
        let wire = r#"{"update_id":7,"callback_query":{"id":"abc","from":{"id":99,"is_bot":false},"data":"approve"}}"#;
        let upd: TelegramWebhookUpdate = serde_json::from_str(wire).unwrap();
        let cb = upd.callback_query.unwrap();
        assert_eq!(cb.id, "abc");
        assert_eq!(cb.from_user_id, 99);
        assert_eq!(cb.data.as_deref(), Some("approve"));
        assert!(upd.message.is_none());
    }

    #[test]
    fn message_round_trips_through_json() {
        let original = TelegramMessage {
            message_id: 1,
            chat_id: 2,
            date: 3,
        };
        let s = serde_json::to_string(&original).unwrap();
        let parsed: TelegramMessage = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed, original);
    }
}
