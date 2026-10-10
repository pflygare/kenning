//! Files uploaded into pages: for now only images, checked by their first
//! bytes rather than by what the browser claims. Stored in the database and
//! served at `/api/files/{id}` to members of the file's organization.

use serde::Serialize;
use serde_json::json;
use sqlx::PgConnection;
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    AppError, AppResult,
    audit::{self, Event},
};

/// Largest file accepted, in bytes.
pub const MAX_SIZE: usize = 10 * 1024 * 1024;

/// An uploaded file; `url` is what page markdown links to.
#[derive(Clone, Debug, Serialize, TS)]
#[ts(export)]
pub struct UploadedFile {
    pub id: Uuid,
    pub url: String,
    pub name: String,
    pub content_type: String,
    pub size: i32,
}

/// The file's type from its first bytes, if it's an image we accept.
pub fn image_type(data: &[u8]) -> Option<&'static str> {
    match data {
        [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, ..] => Some("image/png"),
        [0xff, 0xd8, 0xff, ..] => Some("image/jpeg"),
        [b'G', b'I', b'F', b'8', b'7' | b'9', b'a', ..] => Some("image/gif"),
        [b'R', b'I', b'F', b'F', _, _, _, _, rest @ ..] if rest.starts_with(b"WEBP") => {
            Some("image/webp")
        }
        _ => None,
    }
}

pub fn url(id: Uuid) -> String {
    format!("/api/files/{id}")
}

/// A file name safe to show and to send back in a header.
fn clean_name(name: Option<&str>, content_type: &str) -> String {
    let name: String = name
        .unwrap_or("")
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or("")
        .chars()
        .filter(|c| !c.is_control() && *c != '"')
        .take(200)
        .collect();
    let name = name.trim();
    if name.is_empty() {
        let ext = content_type.trim_start_matches("image/");
        format!("image.{}", if ext == "jpeg" { "jpg" } else { ext })
    } else {
        name.to_string()
    }
}

pub async fn upload(
    conn: &mut PgConnection,
    org_id: Uuid,
    actor_id: Uuid,
    name: Option<&str>,
    data: &[u8],
) -> AppResult<UploadedFile> {
    if data.is_empty() {
        return Err(AppError::BadRequest("The file is empty.".into()));
    }
    if data.len() > MAX_SIZE {
        return Err(too_large());
    }
    let content_type = image_type(data).ok_or_else(|| {
        AppError::coded(
            axum::http::StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported_file",
            "Only PNG, JPEG, GIF and WebP images can be added.",
        )
    })?;
    let id = Uuid::now_v7();
    let name = clean_name(name, content_type);
    let size = data.len() as i32;
    sqlx::query(
        "INSERT INTO files (id, org_id, uploaded_by, name, content_type, size, data)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(id)
    .bind(org_id)
    .bind(actor_id)
    .bind(&name)
    .bind(content_type)
    .bind(size)
    .bind(data)
    .execute(&mut *conn)
    .await?;
    audit::record(
        conn,
        Event::new("file.uploaded")
            .org(org_id)
            .actor(actor_id)
            .object("file", id)
            .details(json!({"name": name, "content_type": content_type, "size": size})),
    )
    .await?;
    Ok(UploadedFile {
        id,
        url: url(id),
        name,
        content_type: content_type.to_string(),
        size,
    })
}

pub fn too_large() -> AppError {
    AppError::coded(
        axum::http::StatusCode::PAYLOAD_TOO_LARGE,
        "file_too_large",
        format!("Images can be at most {} MB.", MAX_SIZE / 1024 / 1024),
    )
}

/// A stored file's name, type and bytes.
pub struct StoredFile {
    pub name: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

/// The file, if `user_id` belongs to its organization.
pub async fn get_for_member(
    conn: &mut PgConnection,
    id: Uuid,
    user_id: Uuid,
) -> AppResult<StoredFile> {
    let row: Option<(String, String, Vec<u8>)> = sqlx::query_as(
        "SELECT f.name, f.content_type, f.data
         FROM files f JOIN memberships m ON m.org_id = f.org_id AND m.user_id = $2
         WHERE f.id = $1",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(conn)
    .await?;
    let (name, content_type, data) = row.ok_or(AppError::NotFound)?;
    Ok(StoredFile {
        name,
        content_type,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn images_are_recognized_by_their_bytes() {
        assert_eq!(image_type(b"\x89PNG\r\n\x1a\n...."), Some("image/png"));
        assert_eq!(image_type(b"\xff\xd8\xff\xe0"), Some("image/jpeg"));
        assert_eq!(image_type(b"GIF89a.."), Some("image/gif"));
        assert_eq!(image_type(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));
        assert_eq!(image_type(b"<svg xmlns="), None);
        assert_eq!(image_type(b"<html>"), None);
    }

    #[test]
    fn names_are_cleaned() {
        assert_eq!(
            clean_name(Some("C:\\x\\cat \"1\".png"), "image/png"),
            "cat 1.png"
        );
        assert_eq!(clean_name(Some("../../etc/passwd"), "image/png"), "passwd");
        assert_eq!(clean_name(None, "image/jpeg"), "image.jpg");
        assert_eq!(clean_name(Some("  "), "image/webp"), "image.webp");
    }
}
