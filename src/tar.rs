use std::io::Write;
use std::path::Path;
use std::{fs, io};
use tar::Builder;

pub(crate) fn append_dir_all_sorted(
    dst: &mut Builder<impl Write>,
    path: &Path,
    src_path: &Path,
) -> io::Result<()> {
    let mut stack = vec![(src_path.to_path_buf(), true, false)];
    while let Some((src, is_dir, is_symlink)) = stack.pop() {
        let dest = path.join(src.strip_prefix(&src_path).unwrap());
        // In case of a symlink pointing to a directory, is_dir is false, but src.is_dir() will return true
        if is_dir || (is_symlink && src.is_dir()) {
            let mut entries = fs::read_dir(&src)?
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?;
            entries.sort_by_key(|x| x.file_name());
            for entry in entries {
                let file_type = entry.file_type()?;
                stack.push((entry.path(), file_type.is_dir(), file_type.is_symlink()));
            }
            if dest != Path::new("") {
                dst.append_dir(&dest, &src)?;
            }
        } else {
            dst.append_file(&dest, &mut fs::File::open(src)?)?;
        }
    }
    Ok(())
}
