use tracing::warn;

use crate::pack_io::reading::{AssetPackReader, FileMeta, TOC};

use super::{ConditionalSendAsyncSeekableBufRead, DirectoryInfo};

/// An iterator that yields all the files (recursive) of a directory in an asset pack.
pub struct IterDir<'a> {
    toc: &'a TOC,
    index: usize,
    dir_name_with_slash: String,
}

impl<'a> Iterator for IterDir<'a> {
    type Item = (&'a String, &'a FileMeta);

    fn next(&mut self) -> Option<Self::Item> {
        self.index += 1;

        let (path, meta) = self.toc.normal_files.get_index(self.index)?;

        if path.starts_with(&self.dir_name_with_slash) {
            Some((path, meta))
        } else {
            None
        }
    }
}

impl<R: ConditionalSendAsyncSeekableBufRead> AssetPackReader<R> {
    /// Returns an iterator of all file paths in a directory.
    ///
    /// NOTE: If the directory name is not cached (16 directories will be cached in an LRU cache at any one time),
    /// this function will iterate through every file in the TOC and checking if they belong to the directory.
    /// Don't use this unless you absolutely have to.
    ///
    /// # Parameters
    /// - `path`: The path of the directory relative to the assets directory (without ./)
    pub async fn iter_directory(&mut self, path: &str) -> Option<IterDir<'_>> {
        if !path.ends_with('/') {
            warn!(
                "`iter_directory` returned `None` because your path does not end with a trailing slash!"
            );
            return None;
        }

        if let DirectoryInfo::Directory(index) = self.get_directory_info(path).await {
            Some(IterDir {
                toc: &self.toc,
                index,
                dir_name_with_slash: path.to_string(),
            })
        } else {
            None
        }
    }
}
