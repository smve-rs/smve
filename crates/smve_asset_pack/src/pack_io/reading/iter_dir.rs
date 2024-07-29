use crate::pack_io::reading::{AssetPackReader, FileMeta, PackFront, ReadError, ReadResult};

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

impl AssetPackReader {
    /// Returns an iterator of all file paths in a directory.
    ///
    /// # Parameters
    /// - `path`: The path of the directory relative to the assets directory (without ./)
    pub fn iter_directory(&mut self, path: &str) -> ReadResult<IterDir> {
        if !self.has_directory(path)? {
            return Err(ReadError::DirectoryNotFound(path.into()));
        }

        let pack_front = self.get_pack_front()?;

        Ok(IterDir {
            pack_front,
            index: *pack_front
                .directory_list
                .get(path)
                .expect("Existence has been checked before."),
            dir_name_with_slash: path.to_owned() + "/",
        })
    }
}
