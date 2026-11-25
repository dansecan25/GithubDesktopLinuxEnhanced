//! Image previews in diffs, as in GitHub Desktop: the picture before and after the change.
//!
//! Decoding runs in background tasks. Huge images are refused before decoding and large ones are
//! shrunk, so a preview can't freeze the window or use up memory.

use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use image::{ImageReader, Limits};

use crate::constants::git::{IMAGE_EXTENSIONS, MAX_PREVIEW_FILE_BYTES, MAX_PREVIEW_PIXELS};
use crate::constants::layout::MAX_PREVIEW_SIDE;
use crate::constants::text;
use crate::git::changes;
use crate::models::{ChangeKind, FileChange, ImagePair, ImageSide, PreviewImage};

pub fn is_image(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| IMAGE_EXTENSIONS.iter().any(|known| known.eq_ignore_ascii_case(extension)))
}

/// A working-tree change: the last commit's version against the file on disk.
pub fn working(repo: &Path, change: &FileChange, has_commits: bool) -> ImagePair {
    let old_path = change.original_path.as_deref().unwrap_or(&change.path);
    let is_new = matches!(change.kind, ChangeKind::Untracked | ChangeKind::Added) || !has_commits;
    let before = if is_new { ImageSide::Missing } else { from_commit(repo, "HEAD", old_path) };
    let after = if change.kind == ChangeKind::Deleted {
        ImageSide::Missing
    } else {
        match read_file(&repo.join(&change.path)) {
            Ok(bytes) => decode(&bytes),
            Err(reason) => ImageSide::Unavailable(reason),
        }
    };
    ImagePair { before, after }
}

/// A file in a commit, against the same file in its first parent.
pub fn in_commit(repo: &Path, sha: &str, path: &str) -> ImagePair {
    ImagePair { before: from_commit(repo, &format!("{sha}^"), path), after: from_commit(repo, sha, path) }
}

fn from_commit(repo: &Path, rev: &str, path: &str) -> ImageSide {
    // A failure here means the file isn't in that commit (added, deleted, or a first commit).
    match changes::file_at(repo, rev, path, MAX_PREVIEW_FILE_BYTES) {
        Ok(bytes) => decode(&bytes),
        Err(error) if error.message.contains("printed more than") => {
            ImageSide::Unavailable(text::IMAGE_TOO_LARGE.to_owned())
        }
        Err(_) => ImageSide::Missing,
    }
}

fn read_file(path: &Path) -> Result<Vec<u8>, String> {
    let size = std::fs::metadata(path).map_err(|error| error.to_string())?.len();
    if size > MAX_PREVIEW_FILE_BYTES as u64 {
        return Err(text::IMAGE_TOO_LARGE.to_owned());
    }
    std::fs::read(path).map_err(|error| error.to_string())
}

/// Decodes an image, shrinking it to at most `MAX_PREVIEW_SIDE` pixels on its longer side.
pub fn decode(bytes: &[u8]) -> ImageSide {
    let reader = || ImageReader::new(Cursor::new(bytes)).with_guessed_format();
    let Ok((width, height)) =
        reader().map_err(|_| ()).and_then(|reader| reader.into_dimensions().map_err(|_| ()))
    else {
        return ImageSide::Unavailable(text::IMAGE_UNREADABLE.to_owned());
    };
    if u64::from(width) * u64::from(height) > MAX_PREVIEW_PIXELS {
        return ImageSide::Unavailable(text::IMAGE_TOO_LARGE.to_owned());
    }
    let Ok(mut reader) = reader() else { return ImageSide::Unavailable(text::IMAGE_UNREADABLE.to_owned()) };
    let mut limits = Limits::default();
    limits.max_alloc = Some(MAX_PREVIEW_PIXELS * 4 * 2);
    reader.limits(limits);
    let Ok(image) = reader.decode() else { return ImageSide::Unavailable(text::IMAGE_UNREADABLE.to_owned()) };
    let image = if width.max(height) > MAX_PREVIEW_SIDE {
        image.thumbnail(MAX_PREVIEW_SIDE, MAX_PREVIEW_SIDE)
    } else {
        image
    };
    let rgba = image.to_rgba8();
    ImageSide::Image(PreviewImage {
        width,
        height,
        file_size: bytes.len(),
        shown_width: rgba.width(),
        shown_height: rgba.height(),
        rgba: Arc::new(rgba.into_raw()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny PNG made with the same crate.
    fn png(width: u32, height: u32) -> Vec<u8> {
        let image = image::RgbaImage::from_pixel(width, height, image::Rgba([255, 0, 0, 128]));
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(image)
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        bytes
    }

    #[test]
    fn recognises_image_files() {
        assert!(is_image("docs/screenshots/changes.PNG"));
        assert!(is_image("photo.jpeg"));
        assert!(!is_image("main.rs"));
        assert!(!is_image("README"));
    }

    #[test]
    fn decodes_and_shrinks_large_images() {
        let ImageSide::Image(small) = decode(&png(3, 2)) else { panic!("not decoded") };
        assert_eq!((small.width, small.height, small.shown_width), (3, 2, 3));
        assert_eq!(small.rgba.len(), 3 * 2 * 4);
        let ImageSide::Image(large) = decode(&png(4096, 1024)) else { panic!("not decoded") };
        assert_eq!((large.width, large.shown_width, large.shown_height), (4096, 2048, 512));
    }

    #[test]
    fn damaged_files_are_reported_not_decoded() {
        assert!(matches!(decode(b"not an image"), ImageSide::Unavailable(_)));
    }
}
