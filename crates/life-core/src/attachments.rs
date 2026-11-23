//! Portable managed attachments stored in the local SQLite database.
//! Limits prevent accidental enormous IPC payloads, and binary bytes never
//! enter the activity history or user-visible metadata queries.
use super::*;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use sha2::{Digest, Sha256};

const MAX_FILE: usize = 8 * 1024 * 1024;
fn file_basename(name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty()
        || name.chars().count() > 220
        || name.chars().any(char::is_control)
        || name.contains('/')
        || name.contains('\\')
        || name == "."
        || name == ".."
    {
        return Err(LifeError::Invalid(
            "filename must be a safe basename of 1–220 characters".into(),
        ));
    }
    Ok(name.to_owned())
}
impl Store {
    pub fn attachments_list(&self, p: &Value) -> Result<Value> {
        let kind = require(p, "owner_kind")?;
        let key = require(p, "owner_id")?;
        self.attachments_owner(&kind, &key)?;
        self.rows("SELECT id,owner_kind,owner_id,filename,mime_type,bytes,sha256,created_at,deleted_at FROM managed_attachments WHERE owner_kind=?1 AND owner_id=?2 AND deleted_at IS NULL ORDER BY created_at DESC LIMIT 200",&[&kind,&key])
    }
    fn attachments_owner(&self, kind: &str, key: &str) -> Result<()> {
        if !["tasks", "notes", "projects", "goals"].contains(&kind) {
            return Err(LifeError::Invalid(
                "Attachments support tasks, notes, projects and goals".into(),
            ));
        }
        self.fetch(kind, key)?;
        Ok(())
    }
    pub fn attachments_create(&self, p: &Value, actor: &str) -> Result<Value> {
        if p.get("request_id").is_some() {
            return Err(LifeError::Invalid("Attachment data must not be written to idempotency payload logs; retry by hash instead".into()));
        }
        let kind = require(p, "owner_kind")?;
        let key = require(p, "owner_id")?;
        self.attachments_owner(&kind, &key)?;
        let name = file_basename(&require(p, "filename")?)?;
        let mime = optional(p, "mime_type").unwrap_or_else(|| "application/octet-stream".into());
        if mime.len() > 120 || mime.chars().any(char::is_control) || !mime.contains('/') {
            return Err(LifeError::Invalid("Invalid MIME type".into()));
        }
        let encoded = require(p, "data_base64")?;
        if encoded.len() > MAX_FILE.div_ceil(3) * 4 + 4 {
            return Err(LifeError::Invalid("Attachment exceeds 8 MiB limit".into()));
        }
        let bytes = STANDARD
            .decode(encoded)
            .map_err(|_| LifeError::Invalid("data_base64 must be valid base64".into()))?;
        if bytes.is_empty() || bytes.len() > MAX_FILE {
            return Err(LifeError::Invalid(
                "Attachment must be between 1 byte and 8 MiB".into(),
            ));
        }
        let hash = format!("{:x}", Sha256::digest(&bytes));
        let duplicate=self.rows("SELECT id,owner_kind,owner_id,filename,mime_type,bytes,sha256,created_at,deleted_at FROM managed_attachments WHERE owner_kind=?1 AND owner_id=?2 AND sha256=?3 AND filename=?4 AND deleted_at IS NULL LIMIT 1",&[&kind,&key,&hash,&name])?;
        if let Some(existing) = duplicate.as_array().and_then(|v| v.first()) {
            return Ok(existing.clone());
        }
        let attachment_id = id();
        let n = bytes.len() as i64;
        self.conn.execute("INSERT INTO managed_attachments(id,owner_kind,owner_id,filename,mime_type,content,bytes,sha256,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",params![attachment_id,kind,key,name,mime,bytes,n,hash,now()])?;
        self.record(
            actor,
            "attachments.create",
            Some(&attachment_id),
            &json!({"owner_kind":kind,"owner_id":key,"filename":name,"bytes":n,"sha256":hash}),
        )?;
        self.attachment_metadata(&attachment_id)
    }
    fn attachment_metadata(&self, key: &str) -> Result<Value> {
        self.rows("SELECT id,owner_kind,owner_id,filename,mime_type,bytes,sha256,created_at,deleted_at FROM managed_attachments WHERE id=?1",&[&key])
            .and_then(|v|v.as_array().and_then(|v|v.first()).cloned().ok_or_else(||LifeError::NotFound(key.to_string())))
    }
    pub fn attachment_get(&self, p: &Value) -> Result<Value> {
        let key = require(p, "id")?;
        let row=self.conn.query_row("SELECT filename,mime_type,content,sha256,deleted_at FROM managed_attachments WHERE id=?1",[&key],|r|{
            Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Vec<u8>>(2)?,r.get::<_,String>(3)?,r.get::<_,Option<String>>(4)?))
        }).optional()?;
        let (filename, mime, bytes, hash, deleted_at) =
            row.ok_or_else(|| LifeError::NotFound(key.clone()))?;
        if deleted_at.is_some() {
            return Err(LifeError::NotFound(key));
        }
        let actual = format!("{:x}", Sha256::digest(&bytes));
        if actual != hash {
            return Err(LifeError::Conflict(
                "Stored attachment checksum does not match".into(),
            ));
        }
        Ok(
            json!({"id":key,"filename":filename,"mime_type":mime,"bytes":bytes.len(),"sha256":hash,"data_base64":STANDARD.encode(&bytes)}),
        )
    }
    pub fn attachment_delete(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let n = self.conn.execute(
            "UPDATE managed_attachments SET deleted_at=?2 WHERE id=?1 AND deleted_at IS NULL",
            params![key, now()],
        )?;
        if n == 0 {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "attachments.delete", Some(&key), &json!({"id":key}))?;
        Ok(json!({"archived":true,"id":key}))
    }
    pub fn attachment_restore(&self, p: &Value, actor: &str) -> Result<Value> {
        let key = require(p, "id")?;
        let n = self.conn.execute(
            "UPDATE managed_attachments SET deleted_at=NULL WHERE id=?1 AND deleted_at IS NOT NULL",
            [&key],
        )?;
        if n == 0 {
            return Err(LifeError::NotFound(key));
        }
        self.record(actor, "attachments.restore", Some(&key), &json!({"id":key}))?;
        self.attachment_metadata(&key)
    }
}
