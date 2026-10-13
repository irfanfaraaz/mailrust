//! Uploaded images. Only this module reads or writes the `images` table; everything else goes
//! through `ImageStore`, so an S3 store later is one new impl and a setting.
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

pub const MAX_IMAGE_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct NewImage {
    pub content_type: &'static str,
    pub bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone)]
pub struct StoredImage {
    pub account_id: Uuid,
    pub content_type: String,
    pub bytes: Vec<u8>,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error(transparent)]
    Db(#[from] sqlx::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImageError {
    #[error("images are at most 5 MB")]
    TooLarge,
    #[error("upload a PNG, JPEG, GIF or WebP image")]
    Unsupported,
    #[error("this image file is damaged")]
    Broken,
}

#[async_trait]
pub trait ImageStore: Send + Sync {
    async fn put(&self, account_id: Uuid, img: NewImage) -> Result<Uuid, StoreError>;
    async fn get(&self, id: Uuid) -> Result<Option<StoredImage>, StoreError>;
    /// Which of these ids belong to this account, without loading the bytes (block checks).
    async fn owned(&self, account_id: Uuid, ids: &[Uuid]) -> Result<Vec<Uuid>, StoreError>;
}

/// The image type from its first bytes. SVG and everything else: None.
pub fn sniff(b: &[u8]) -> Option<&'static str> {
    if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("image/png")
    } else if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("image/jpeg")
    } else if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") {
        Some("image/gif")
    } else if b.len() >= 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        Some("image/webp")
    } else {
        None
    }
}

pub fn inspect(bytes: Vec<u8>) -> Result<NewImage, ImageError> {
    if bytes.len() > MAX_IMAGE_BYTES {
        return Err(ImageError::TooLarge);
    }
    let content_type = sniff(&bytes).ok_or(ImageError::Unsupported)?;
    let size = imagesize::blob_size(&bytes).map_err(|_| ImageError::Broken)?;
    let width = u32::try_from(size.width).map_err(|_| ImageError::Broken)?;
    let height = u32::try_from(size.height).map_err(|_| ImageError::Broken)?;
    if width == 0 || height == 0 {
        return Err(ImageError::Broken);
    }
    Ok(NewImage {
        content_type,
        bytes,
        width,
        height,
    })
}

pub struct PgImageStore {
    pool: PgPool,
}

impl PgImageStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ImageStore for PgImageStore {
    async fn put(&self, account_id: Uuid, img: NewImage) -> Result<Uuid, StoreError> {
        let id = Uuid::now_v7();
        sqlx::query!(
            "INSERT INTO images (id, account_id, content_type, bytes, size, width, height) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            id,
            account_id,
            img.content_type,
            img.bytes,
            img.bytes.len() as i32,
            img.width as i32,
            img.height as i32
        )
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    async fn get(&self, id: Uuid) -> Result<Option<StoredImage>, StoreError> {
        Ok(sqlx::query_as!(
            StoredImage,
            "SELECT account_id, content_type, bytes FROM images WHERE id = $1",
            id
        )
        .fetch_optional(&self.pool)
        .await?)
    }

    async fn owned(&self, account_id: Uuid, ids: &[Uuid]) -> Result<Vec<Uuid>, StoreError> {
        Ok(sqlx::query_scalar!(
            "SELECT id FROM images WHERE id = ANY($1) AND account_id = $2",
            ids,
            account_id
        )
        .fetch_all(&self.pool)
        .await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine, engine::general_purpose::STANDARD};

    const PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

    #[test]
    fn the_type_comes_from_the_magic_bytes() {
        assert_eq!(sniff(&STANDARD.decode(PNG).unwrap()), Some("image/png"));
        assert_eq!(sniff(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 0]), Some("image/jpeg"));
        assert_eq!(sniff(b"GIF89a\x01\x00"), Some("image/gif"));
        assert_eq!(sniff(b"GIF87a\x01\x00"), Some("image/gif"));
        assert_eq!(sniff(b"RIFF\x24\x00\x00\x00WEBPVP8 "), Some("image/webp"));
        assert_eq!(sniff(b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>"), None);
        assert_eq!(sniff(b"<?xml version=\"1.0\"?><svg/>"), None);
        assert_eq!(sniff(b"RIFF\x24\x00\x00\x00WAVEfmt "), None);
        assert_eq!(sniff(b""), None);
    }

    #[test]
    fn inspect_reads_the_size_and_refuses_the_rest() {
        let img = inspect(STANDARD.decode(PNG).unwrap()).unwrap();
        assert_eq!(
            (img.content_type, img.width, img.height),
            ("image/png", 1, 1)
        );
        assert_eq!(
            inspect(b"<svg/>".to_vec()).unwrap_err(),
            ImageError::Unsupported
        );
        assert_eq!(
            inspect(b"\x89PNG\r\n\x1a\nbroken".to_vec()).unwrap_err(),
            ImageError::Broken
        );
        let mut big = STANDARD.decode(PNG).unwrap();
        big.resize(MAX_IMAGE_BYTES + 1, 0);
        assert_eq!(inspect(big).unwrap_err(), ImageError::TooLarge);
    }
}
