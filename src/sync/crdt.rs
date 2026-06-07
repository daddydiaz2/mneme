use std::str::FromStr;

use automerge::transaction::Transactable;
use automerge::{AutoCommit, Automerge, ObjId, ReadDoc, ScalarValue, Value};
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::error::{MnemeError, Result};
use crate::store::memory::{Importance, Memory, MemoryType, Scope};

/// Convierte una memoria a documento automerge.
pub fn memory_to_doc(memory: &Memory) -> Result<AutoCommit> {
    let mut doc = AutoCommit::new();
    let root = ObjId::Root;

    doc.put(&root, "id", memory.id.to_string())?;
    doc.put(&root, "project", memory.project.clone())?;
    doc.put(&root, "scope", memory.scope.to_string())?;
    doc.put(&root, "title", memory.title.clone())?;
    doc.put(&root, "content", memory.content.clone())?;
    if let Some(ref what) = memory.what {
        doc.put(&root, "what", what.clone())?;
    }
    if let Some(ref why) = memory.why {
        doc.put(&root, "why", why.clone())?;
    }
    if let Some(ref context) = memory.context {
        doc.put(&root, "context", context.clone())?;
    }
    if let Some(ref learned) = memory.learned {
        doc.put(&root, "learned", learned.clone())?;
    }
    doc.put(&root, "memory_type", memory.memory_type.to_string())?;
    doc.put(&root, "importance", memory.importance.to_string())?;
    doc.put(&root, "tags", serde_json::to_string(&memory.tags)?)?;
    if let Some(ref topic_key) = memory.topic_key {
        doc.put(&root, "topic_key", topic_key.clone())?;
    }
    doc.put(
        &root,
        "access_count",
        ScalarValue::Int(memory.access_count as i64),
    )?;
    doc.put(
        &root,
        "revision_count",
        ScalarValue::Int(memory.revision_count as i64),
    )?;
    doc.put(
        &root,
        "duplicate_count",
        ScalarValue::Int(memory.duplicate_count as i64),
    )?;
    if let Some(ref hash) = memory.normalized_hash {
        doc.put(&root, "normalized_hash", hash.clone())?;
    }
    doc.put(&root, "created_at", memory.created_at.to_rfc3339())?;
    doc.put(&root, "updated_at", memory.updated_at.to_rfc3339())?;
    if let Some(ref last_accessed) = memory.last_accessed_at {
        doc.put(&root, "last_accessed_at", last_accessed.to_rfc3339())?;
    }
    if let Some(ref last_seen) = memory.last_seen_at {
        doc.put(&root, "last_seen_at", last_seen.to_rfc3339())?;
    }
    if let Some(ref deleted_at) = memory.deleted_at {
        doc.put(&root, "deleted_at", deleted_at.to_rfc3339())?;
    }
    if let Some(ref deprecated_at) = memory.deprecated_at {
        doc.put(&root, "deprecated_at", deprecated_at.to_rfc3339())?;
    }
    if let Some(ref deprecated_reason) = memory.deprecated_reason {
        doc.put(&root, "deprecated_reason", deprecated_reason.clone())?;
    }
    if let Some(ref supersedes_id) = memory.supersedes_id {
        doc.put(&root, "supersedes_id", supersedes_id.clone())?;
    }
    doc.put(
        &root,
        "context_inject_count",
        ScalarValue::Int(memory.context_inject_count as i64),
    )?;

    Ok(doc)
}

/// Convierte bytes de documento automerge a memoria.
pub fn doc_to_memory(doc_bytes: &[u8]) -> Result<Memory> {
    let doc = Automerge::load(doc_bytes)?;
    let root = ObjId::Root;

    let get_str = |key: &str| -> Result<String> {
        match doc.get(&root, key) {
            Ok(Some((Value::Scalar(v), _))) => match v.to_str() {
                Some(s) => Ok(s.to_string()),
                None => Err(MnemeError::Config(format!("campo {} no es string", key))),
            },
            _ => Err(MnemeError::Config(format!("campo {} no encontrado", key))),
        }
    };

    let get_opt_str = |key: &str| -> Result<Option<String>> {
        match doc.get(&root, key) {
            Ok(Some((Value::Scalar(v), _))) => match v.to_str() {
                Some(s) => Ok(Some(s.to_string())),
                None => Ok(None),
            },
            _ => Ok(None),
        }
    };

    let get_int = |key: &str| -> Result<u32> {
        match doc.get(&root, key) {
            Ok(Some((Value::Scalar(v), _))) => match v.as_ref() {
                ScalarValue::Int(i) => Ok(*i as u32),
                ScalarValue::Uint(i) => Ok(*i as u32),
                ScalarValue::F64(f) => Ok(*f as u32),
                _ => Ok(0),
            },
            _ => Ok(0),
        }
    };

    let get_date = |key: &str| -> Result<Option<DateTime<Utc>>> {
        match doc.get(&root, key) {
            Ok(Some((Value::Scalar(v), _))) => match v.to_str() {
                Some(s) => match DateTime::parse_from_rfc3339(s) {
                    Ok(d) => Ok(Some(d.with_timezone(&Utc))),
                    Err(_) => Ok(None),
                },
                None => Ok(None),
            },
            _ => Ok(None),
        }
    };

    let id = Uuid::parse_str(&get_str("id")?)
        .map_err(|e| MnemeError::Config(format!("uuid invalido: {}", e)))?;
    let tags: Vec<String> =
        serde_json::from_str(&get_str("tags").unwrap_or_else(|_| "[]".to_string()))?;

    Ok(Memory {
        id,
        project: get_str("project")?,
        scope: Scope::from_str(&get_str("scope")?)?,
        title: get_str("title")?,
        content: get_str("content")?,
        what: get_opt_str("what")?,
        why: get_opt_str("why")?,
        context: get_opt_str("context")?,
        learned: get_opt_str("learned")?,
        memory_type: MemoryType::from_str(&get_str("memory_type")?)?,
        importance: Importance::from_str(&get_str("importance")?)?,
        tags,
        topic_key: get_opt_str("topic_key")?,
        access_count: get_int("access_count")?,
        revision_count: get_int("revision_count")?,
        duplicate_count: get_int("duplicate_count")?,
        normalized_hash: get_opt_str("normalized_hash")?,
        created_at: get_date("created_at")?.unwrap_or_else(Utc::now),
        updated_at: get_date("updated_at")?.unwrap_or_else(Utc::now),
        last_accessed_at: get_date("last_accessed_at")?,
        last_seen_at: get_date("last_seen_at")?,
        deleted_at: get_date("deleted_at")?,
        deprecated_at: get_date("deprecated_at")?,
        deprecated_reason: get_opt_str("deprecated_reason")?,
        supersedes_id: get_opt_str("supersedes_id")?,
        context_inject_count: get_int("context_inject_count")?,
        origin_peer: None,
        is_encrypted: get_int("is_encrypted").map(|v| v != 0).unwrap_or(false),
        encrypted_for: get_opt_str("encrypted_for")?,
        valid_from: get_date("valid_from")?,
        valid_until: get_date("valid_until")?,
        provenance: get_opt_str("provenance")?,
    })
}

/// Obtiene los heads (hashes de cambio) de un documento.
pub fn get_heads(doc_bytes: &[u8]) -> Result<Vec<String>> {
    let doc = Automerge::load(doc_bytes)?;
    Ok(doc.get_heads().iter().map(|h| h.to_string()).collect())
}

/// Serializa un documento automerge a bytes.
pub fn doc_to_bytes(doc: &mut AutoCommit) -> Result<Vec<u8>> {
    Ok(doc.save_incremental())
}

/// Combina dos documentos automerge.
pub fn merge_docs(base: &[u8], incoming: &[u8]) -> Result<Vec<u8>> {
    let mut base_doc = Automerge::load(base)?;
    let incoming_doc = Automerge::load(incoming)?;
    base_doc.merge(&mut incoming_doc.clone())?;
    Ok(base_doc.save())
}
