use blake3::Hasher;
use std::io::Write;

pub trait WriteExt {
    fn write_all_and_hash(&mut self, buf: &[u8], hasher: &mut Hasher) -> std::io::Result<()>;
}

impl<W> WriteExt for W
where
    W: Write,
{
    fn write_all_and_hash(&mut self, buf: &[u8], hasher: &mut Hasher) -> std::io::Result<()> {
        hasher.update(buf);
        self.write_all(buf)?;
        Ok(())
    }
}
