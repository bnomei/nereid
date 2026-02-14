// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Nereid-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Nereid and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use crate::model::ids::ObjectId;
use crate::model::seq_ast::{SequenceAst, SequenceMessage};
use regex::RegexBuilder;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageSearchMode {
    Substring,
    Regex,
}

pub fn message_search<'a>(
    ast: &'a SequenceAst,
    needle: &str,
    mode: MessageSearchMode,
    case_insensitive: bool,
) -> Result<Vec<&'a SequenceMessage>, regex::Error> {
    match mode {
        MessageSearchMode::Substring => {
            if case_insensitive {
                let needle_lower = needle.to_lowercase();
                Ok(ast
                    .messages_in_order()
                    .into_iter()
                    .filter(|msg| msg.text().to_lowercase().contains(&needle_lower))
                    .collect())
            } else {
                Ok(ast
                    .messages_in_order()
                    .into_iter()
                    .filter(|msg| msg.text().contains(needle))
                    .collect())
            }
        }
        MessageSearchMode::Regex => {
            let regex = RegexBuilder::new(needle)
                .case_insensitive(case_insensitive)
                .build()?;
            Ok(ast
                .messages_in_order()
                .into_iter()
                .filter(|msg| regex.is_match(msg.text()))
                .collect())
        }
    }
}

pub fn messages_between<'a>(
    ast: &'a SequenceAst,
    from_participant_id: &ObjectId,
    to_participant_id: &ObjectId,
) -> Vec<&'a SequenceMessage> {
    ast.messages_in_order()
        .into_iter()
        .filter(|msg| {
            msg.from_participant_id() == from_participant_id
                && msg.to_participant_id() == to_participant_id
        })
        .collect()
}

pub fn trace_before<'a>(
    ast: &'a SequenceAst,
    message_id: &ObjectId,
    limit: usize,
) -> Option<Vec<&'a SequenceMessage>> {
    let messages = ast.messages_in_order();
    let target_index = messages
        .iter()
        .position(|msg| msg.message_id() == message_id)?;
    let start_index = target_index.saturating_sub(limit);
    Some(messages[start_index..target_index].to_vec())
}

pub fn trace_after<'a>(
    ast: &'a SequenceAst,
    message_id: &ObjectId,
    limit: usize,
) -> Option<Vec<&'a SequenceMessage>> {
    let messages = ast.messages_in_order();
    let target_index = messages
        .iter()
        .position(|msg| msg.message_id() == message_id)?;

    let start_index = target_index.saturating_add(1);
    let end_index = start_index.saturating_add(limit).min(messages.len());
    Some(messages[start_index..end_index].to_vec())
}

#[cfg(test)]
mod tests {
    use super::{message_search, messages_between, trace_after, trace_before, MessageSearchMode};
    use crate::model::ids::ObjectId;
    use crate::model::seq_ast::{
        SequenceAst, SequenceMessage, SequenceMessageKind, SequenceParticipant,
    };

    fn message_ids(messages: &[&SequenceMessage]) -> Vec<String> {
        messages
            .iter()
            .map(|msg| msg.message_id().as_str().to_owned())
            .collect()
    }

    fn fixture_ast() -> SequenceAst {
        let mut ast = SequenceAst::default();

        let p_alice = ObjectId::new("p:alice").expect("participant id");
        let p_bob = ObjectId::new("p:bob").expect("participant id");
        let p_carol = ObjectId::new("p:carol").expect("participant id");

        ast.participants_mut()
            .insert(p_alice.clone(), SequenceParticipant::new("Alice"));
        ast.participants_mut()
            .insert(p_bob.clone(), SequenceParticipant::new("Bob"));
        ast.participants_mut()
            .insert(p_carol.clone(), SequenceParticipant::new("Carol"));

        let m_0001 = ObjectId::new("m:0001").expect("message id");
        let m_0002 = ObjectId::new("m:0002").expect("message id");
        let m_0003 = ObjectId::new("m:0003").expect("message id");
        let m_0004 = ObjectId::new("m:0004").expect("message id");

        // Intentionally insert out of order to validate deterministic ordering in queries.
        ast.messages_mut().push(SequenceMessage::new(
            m_0003.clone(),
            p_bob.clone(),
            p_alice.clone(),
            SequenceMessageKind::Return,
            "Bye",
            2000,
        ));
        ast.messages_mut().push(SequenceMessage::new(
            m_0002.clone(),
            p_alice.clone(),
            p_bob.clone(),
            SequenceMessageKind::Sync,
            "Hello there",
            1000,
        ));
        ast.messages_mut().push(SequenceMessage::new(
            m_0001.clone(),
            p_alice.clone(),
            p_bob.clone(),
            SequenceMessageKind::Sync,
            "Hello",
            2000,
        ));
        ast.messages_mut().push(SequenceMessage::new(
            m_0004.clone(),
            p_alice.clone(),
            p_carol.clone(),
            SequenceMessageKind::Async,
            "After",
            3000,
        ));

        ast
    }

    #[test]
    fn message_search_is_deterministic_and_ordered() {
        let ast = fixture_ast();
        let hits = message_search(&ast, "Hello", MessageSearchMode::Substring, true)
            .expect("search result");
        assert_eq!(message_ids(&hits), vec!["m:0002", "m:0001"]);
    }

    #[test]
    fn message_search_can_be_case_insensitive_in_substring_mode() {
        let ast = fixture_ast();
        let hits = message_search(&ast, "hello", MessageSearchMode::Substring, true)
            .expect("search result");
        assert_eq!(message_ids(&hits), vec!["m:0002", "m:0001"]);

        let hits = message_search(&ast, "hello", MessageSearchMode::Substring, false)
            .expect("search result");
        assert!(hits.is_empty());
    }

    #[test]
    fn message_search_supports_regex_mode() {
        let ast = fixture_ast();
        let hits =
            message_search(&ast, "^hello", MessageSearchMode::Regex, true).expect("search result");
        assert_eq!(message_ids(&hits), vec!["m:0002", "m:0001"]);
    }

    #[test]
    fn message_search_returns_error_for_invalid_regex() {
        let ast = fixture_ast();
        let err = message_search(&ast, "(", MessageSearchMode::Regex, true)
            .expect_err("expected regex compile error");
        let msg = err.to_string();
        assert!(!msg.is_empty());
        assert!(msg.to_lowercase().contains("regex"));
    }

    #[test]
    fn messages_between_filters_by_from_to_in_order() {
        let ast = fixture_ast();
        let from = ObjectId::new("p:alice").expect("participant id");
        let to = ObjectId::new("p:bob").expect("participant id");
        let hits = messages_between(&ast, &from, &to);
        assert_eq!(message_ids(&hits), vec!["m:0002", "m:0001"]);
    }

    #[test]
    fn trace_before_returns_messages_before_in_order() {
        let ast = fixture_ast();
        let target = ObjectId::new("m:0003").expect("message id");
        let trace = trace_before(&ast, &target, 2).expect("trace");
        assert_eq!(message_ids(&trace), vec!["m:0002", "m:0001"]);
    }

    #[test]
    fn trace_after_returns_messages_after_in_order() {
        let ast = fixture_ast();
        let target = ObjectId::new("m:0001").expect("message id");
        let trace = trace_after(&ast, &target, 10).expect("trace");
        assert_eq!(message_ids(&trace), vec!["m:0003", "m:0004"]);
    }

    #[test]
    fn trace_returns_none_when_message_missing() {
        let ast = fixture_ast();
        let missing = ObjectId::new("m:9999").expect("message id");
        assert_eq!(trace_before(&ast, &missing, 1), None);
        assert_eq!(trace_after(&ast, &missing, 1), None);
    }
}
