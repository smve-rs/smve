use std::io::{BufRead, Seek};

use crate::pack_io::reading::{AssetPackReader, FileMeta, PackFront, ReadResult};

/// An iterator that yields all the files (recursive) of a directory in an asset pack.
pub struct IterDir<'a> {
    pack_front: &'a PackFront,
    index: usize,
    dir_name_with_slash: String,
}

impl<'a> Iterator for IterDir<'a> {
    type Item = (&'a String, &'a FileMeta);

    fn next(&mut self) -> Option<Self::Item> {
        self.index += 1;

        let (path, meta) = self.pack_front.toc.get_index(self.index)?;

        if path.starts_with(&self.dir_name_with_slash) {
            Some((path, meta))
        } else {
            None
        }
    }
}

impl<R: BufRead + Seek> AssetPackReader<R> {
    /// Returns an iterator of all file paths in a directory.
    ///
    /// # Parameters
    /// - `path`: The path of the directory relative to the assets directory (without ./)
    pub fn iter_directory(&mut self, path: &str) -> ReadResult<Option<IterDir>> {
        if !self.has_directory(path)? {
            return Ok(None);
        }

        let pack_front = self.get_pack_front()?;

        Ok(Some(IterDir {
            pack_front,
            index: *pack_front
                .directory_list
                .get(path)
                .expect("Existence has been checked before."),
            dir_name_with_slash: path.to_owned() + "/",
        }))
    }
}
