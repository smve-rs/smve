/*
 * RustyCraft: a voxel engine written in Rust
 * Copyright (C)  2023  SunnyMonster
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

pub mod file_bundle {

    use std::error::Error;
    use std::fs::{File, OpenOptions};
    use std::io;
    use std::io::{Seek, Write};

    use std::path::PathBuf;
    use walkdir::WalkDir;

    pub const FILE_HEADER: &[u8; 22] = b"\x00\x00\x69\x42FILEBUNDLEv0.0\x42\x69\x00\x00";

    #[derive(Debug)]
    pub enum CompileStatus {
        Adding,
        Added,
        SkippedDirectory,
        WritingFile
    }

    pub fn compile<F>(
        source: &PathBuf,
        dest: &PathBuf,
        mut progress_callback: F
    ) -> Result<(), Box<dyn Error>>
    where
        //       CompileStatus, name, index, total
        F: FnMut(CompileStatus, &str, usize, usize),
    {
        let mut bundle = File::create(dest)?;
        bundle.write_all(FILE_HEADER)?;
        let mut data_begins = FILE_HEADER.len() as u64;

        let mut temp = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .read(true)
            .open(format!("{}.tmp", dest.display()))?;

        let mut table_of_contents: Vec<(String, u64)> = Vec::new();

        let file_count = WalkDir::new(source).follow_links(true).into_iter().count();

        let mut file_index: usize = 1;
        for entry in WalkDir::new(source).follow_links(true) {
            let entry = entry?;

            let relative_path_str = entry
                .path()
                .strip_prefix(source)
                .expect(format!("Path does not start with {}!", source.display()).as_str())
                .display()
                .to_string();

            if entry.path().is_dir() {
                //println!("Skipping directory");
                progress_callback(CompileStatus::SkippedDirectory, relative_path_str.as_str(), file_index, file_count);
                file_index += 1;
                continue;
            }

            progress_callback(CompileStatus::Adding, relative_path_str.as_str(), file_index, file_count);

            data_begins += relative_path_str.len() as u64;
            data_begins += 1;
            data_begins += 8;
            table_of_contents.push((relative_path_str.clone(), temp.stream_position()?));

            let mut file = File::open(entry.path())?;

            temp.write_all(relative_path_str.as_bytes())?;
            temp.write_all(b"\x00")?;

            temp.write_all(&file.metadata()?.len().to_be_bytes())?;

            io::copy(&mut file, &mut temp)?;
            //println!("{}", temp.stream_position()?);

            progress_callback(CompileStatus::Added, relative_path_str.as_str(), file_index, file_count);

            file_index += 1;
        }

        progress_callback(CompileStatus::WritingFile, "", 0, 0);

        for (name, offset) in table_of_contents {
            bundle.write_all(name.as_bytes())?;
            bundle.write_all(b"\x00")?;
            let offset = offset + data_begins;
            bundle.write_all(&offset.to_be_bytes())?;
        }

        temp.rewind()?;
        io::copy(&mut temp, &mut bundle)?;

        Ok(())
    }
}
