use super::{storage::private_dir, *};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};

/// v1 per-body limit; over-limit inputs are never truncated into a stored body.
pub const MAX_BODY_BYTES: u64 = 16 * 1024 * 1024;

pub(super) struct StagedBody {
    pub reference: Value,
    file: tempfile::NamedTempFile,
}

impl StagedBody {
    pub(super) fn available(&self) -> Result<bool> {
        match fs::symlink_metadata(self.file.path()) {
            Ok(metadata) => {
                Ok(metadata.is_file() && Some(metadata.len()) == self.reference["bytes"].as_u64())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error.into()),
        }
    }
}

fn regular(path: &Path) -> Result<File> {
    if !path.is_absolute() {
        return Err(Error::invalid("body path must be absolute"));
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| Error::new("invalid", "unreadable", "body is not readable"))?;
    if !file.metadata()?.is_file() {
        return Err(Error::new(
            "invalid",
            "unreadable",
            "body must be a regular file",
        ));
    }
    Ok(file)
}

fn checked_bytes(path: &Path, hash: &str, bytes: u64) -> Result<Vec<u8>> {
    let mut input = regular(path).map_err(|_| {
        Error::new(
            "unavailable",
            "body_missing",
            "body file is missing or unreadable",
        )
    })?;
    if input.metadata()?.len() != bytes || bytes > MAX_BODY_BYTES {
        return Err(Error::new(
            "unavailable",
            "body_corrupt",
            "body length mismatch",
        ));
    }
    let mut data = Vec::with_capacity(bytes as usize);
    (&mut input)
        .take(MAX_BODY_BYTES + 1)
        .read_to_end(&mut data)?;
    if data.len() as u64 != bytes || format!("{:x}", Sha256::digest(&data)) != hash {
        return Err(Error::new(
            "unavailable",
            "body_corrupt",
            "body hash mismatch",
        ));
    }
    Ok(data)
}

impl Store {
    pub(super) fn stage(&self, input: &BodyInput) -> Result<StagedBody> {
        let mut source = regular(&input.path)?;
        if source.metadata()?.len() > MAX_BODY_BYTES {
            return Err(Error::new("invalid", "too_large", "body exceeds 16 MiB"));
        }
        private_dir(&self.root.join("tmp"))?;
        let mut file = tempfile::NamedTempFile::new_in(self.root.join("tmp"))?;
        let mut hash = Sha256::new();
        let mut bytes = 0_u64;
        let mut buffer = [0_u8; 65536];
        loop {
            let count = source
                .read(&mut buffer)
                .map_err(|_| Error::new("invalid", "unreadable", "body read failed"))?;
            if count == 0 {
                break;
            }
            bytes += count as u64;
            if bytes > MAX_BODY_BYTES {
                return Err(Error::new("invalid", "too_large", "body exceeds 16 MiB"));
            }
            hash.update(&buffer[..count]);
            file.write_all(&buffer[..count])?;
        }
        file.as_file().sync_all()?;
        let hash = format!("{:x}", hash.finalize());
        let destination = self.root.join("blobs/sha256").join(&hash);
        if destination.try_exists()? {
            checked_bytes(&destination, &hash, bytes)?;
        }
        Ok(StagedBody {
            reference: json!({"role":input.role,"sha256":hash,"bytes":bytes}),
            file,
        })
    }

    // Called only while holding the same SQLite write transaction used by switches.
    pub(super) fn publish(&self, conn: &Connection, body: &StagedBody) -> Result<()> {
        let hash = body.reference["sha256"].as_str().expect("staged hash");
        let bytes = body.reference["bytes"].as_u64().expect("staged length") as i64;
        let dir = self.root.join("blobs/sha256");
        private_dir(&dir)?;
        match fs::hard_link(body.file.path(), dir.join(hash)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                // Existing objects were verified before acquiring the write lock. A concurrent
                // publisher uses this same immutable hash path and complete-file protocol.
                let metadata = fs::symlink_metadata(dir.join(hash))?;
                if !metadata.is_file() || metadata.len() != bytes as u64 {
                    return Err(Error::new(
                        "unavailable",
                        "body_corrupt",
                        "existing body length mismatch",
                    ));
                }
            }
            Err(error) => return Err(error.into()),
        }
        File::open(&dir)?.sync_all()?;
        File::open(self.root.join("blobs"))?.sync_all()?;
        File::open(&self.root)?.sync_all()?;
        conn.execute(
            "INSERT INTO blobs(sha256,bytes) VALUES(?,?) ON CONFLICT(sha256) DO NOTHING",
            rusqlite::params![hash, bytes],
        )?;
        Ok(())
    }

    pub fn body(&self, hash: &str) -> Result<Vec<u8>> {
        use rusqlite::OptionalExtension;
        if hash.len() != 64
            || !hash
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        {
            return Err(Error::invalid("sha256 must be 64 lowercase hex characters"));
        }
        let conn = self.reader()?.ok_or_else(|| {
            Error::new(
                "unavailable",
                "not_initialized",
                "telemetry is not initialized",
            )
        })?;
        let bytes: i64 = conn
            .query_row("SELECT bytes FROM blobs WHERE sha256=?", [hash], |r| {
                r.get(0)
            })
            .optional()?
            .ok_or_else(|| Error::new("invalid", "not_found", "unknown body hash"))?;
        checked_bytes(
            &self.root.join("blobs/sha256").join(hash),
            hash,
            bytes as u64,
        )
    }
}
