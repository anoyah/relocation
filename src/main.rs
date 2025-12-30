use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Local};
use clap::{Parser, builder::ValueHint};
use env_logger::{Builder, Env};
use log::{debug, error, info};
use rayon::{ThreadPoolBuilder, prelude::*};
use walkdir::WalkDir;

// 过滤文件
const IGNORE_FILENAME: [&str; 2] = ["Thumbs.db", ".DS_Store"];

// 允许的文件后缀
const SUFFIX: [&str; 21] = [
    "jpg", "mp4", "png", "gif", "bmp", "tiff", "tif", "webp", "svg", "ico", "mp3", "wav", "aac",
    "ogg", "flac", "m4a", "mov", "avi", "wmv", "mkv", "webm",
];

#[derive(Parser, Debug)]
#[command(version, about, long_about = None, arg_required_else_help = true)]
struct Cli {
    /// 设置转移原目录
    #[arg(short, long, value_hint = ValueHint::DirPath)]
    source: PathBuf,
    /// 设置转移目标目录
    #[arg(short, long, value_hint = ValueHint::DirPath)]
    dest: PathBuf,
    /// 并行任务数，默认自动
    #[arg(short = 'j', long)]
    jobs: Option<usize>,
}

#[derive(Default, Debug)]
struct CopyStats {
    processed: usize,
    copied: usize,
    skipped: usize,
    errors: usize,
}

#[derive(Debug)]
enum CopyOutcome {
    Copied,
    SkippedExisting,
}

// `relo -s /path/to/source -d /path/to/dest`
fn main() -> Result<()> {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    let cli: Cli = Cli::parse();

    let stats = relocate(&cli.source, &cli.dest, cli.jobs)?;

    info!(
        "处理完成：总文件 {}，复制 {}，跳过 {}，失败 {}",
        stats.processed, stats.copied, stats.skipped, stats.errors
    );

    Ok(())
}

fn relocate(source: &Path, dest: &Path, jobs: Option<usize>) -> Result<CopyStats> {
    if !source.exists() {
        return Err(anyhow!("目录不存在: {}", source.display()));
    }

    if !source.is_dir() {
        return Err(anyhow!("路径不是一个文件夹: {}", source.display()));
    }

    let mut initial_errors = 0usize;
    let mut files: Vec<PathBuf> = Vec::new();

    for entry in WalkDir::new(source).into_iter() {
        match entry {
            Ok(entry) if entry.file_type().is_file() => files.push(entry.into_path()),
            Ok(_) => {}
            Err(err) => {
                error!("读取目录失败: {}", err);
                initial_errors += 1;
            }
        }
    }

    let work = || {
        files
            .par_iter()
            .map(|path| process_path(path, dest))
            .fold(CopyStats::default, |mut acc, res| {
                acc.processed += 1;
                match res {
                    FileResult::SkippedUnsupported => {
                        acc.skipped += 1;
                    }
                    FileResult::Outcome(CopyOutcome::Copied) => {
                        acc.copied += 1;
                    }
                    FileResult::Outcome(CopyOutcome::SkippedExisting) => {
                        acc.skipped += 1;
                    }
                    FileResult::Failed(path, err) => {
                        error!("复制失败 {} -> {}: {}", path.display(), dest.display(), err);
                        acc.errors += 1;
                    }
                }
                acc
            })
            .reduce(CopyStats::default, |mut a, b| {
                a.processed += b.processed;
                a.copied += b.copied;
                a.skipped += b.skipped;
                a.errors += b.errors;
                a
            })
    };

    let stats = match jobs {
        Some(num) if num > 0 => ThreadPoolBuilder::new()
            .num_threads(num)
            .build()?
            .install(work),
        _ => work(),
    };

    Ok(CopyStats {
        errors: stats.errors + initial_errors,
        ..stats
    })
}

#[derive(Debug)]
enum FileResult {
    Outcome(CopyOutcome),
    SkippedUnsupported,
    Failed(PathBuf, anyhow::Error),
}

fn process_path(path: &Path, dest: &Path) -> FileResult {
    if should_skip(path) {
        debug!("跳过不支持的文件: {}", path.display());
        return FileResult::SkippedUnsupported;
    }

    match copy_single(path, dest) {
        Ok(outcome) => FileResult::Outcome(outcome),
        Err(err) => FileResult::Failed(path.to_path_buf(), err),
    }
}

fn should_skip(path: &Path) -> bool {
    let file_name = match path.file_name().and_then(|f| f.to_str()) {
        Some(name) => name,
        None => return true,
    };

    if IGNORE_FILENAME
        .iter()
        .any(|ignored| file_name.eq_ignore_ascii_case(ignored))
    {
        return true;
    }

    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => !SUFFIX
            .iter()
            .any(|allowed| ext.eq_ignore_ascii_case(allowed)),
        None => true,
    }
}

fn copy_single(source: &Path, dest_root: &Path) -> Result<CopyOutcome> {
    let (file_name, datetime) = get_file_info(source)?;

    let dest_dir = build_dest_dir(dest_root, datetime)?;
    let dest_filename = dest_dir.join(&file_name);

    if dest_filename.exists() {
        info!("目标文件已存在: {}", dest_filename.display());
        return Ok(CopyOutcome::SkippedExisting);
    }

    // 同分区时硬链接比字节拷贝更快，失败再回退到复制
    match fs::hard_link(source, &dest_filename) {
        Ok(_) => {
            info!(
                "源文件:{} \n目标文件:{} (硬链接)",
                source.display(),
                dest_filename.display()
            );
            return Ok(CopyOutcome::Copied);
        }
        Err(err) => {
            debug!(
                "硬链接失败，回退到复制 {} -> {}: {}",
                source.display(),
                dest_filename.display(),
                err
            );
        }
    }

    fs::copy(source, &dest_filename).with_context(|| {
        format!(
            "无法复制 {} 到 {}",
            source.display(),
            dest_filename.display()
        )
    })?;

    info!(
        "源文件:{} \n目标文件:{}",
        source.display(),
        dest_filename.display()
    );

    Ok(CopyOutcome::Copied)
}

fn get_file_info(source: &Path) -> Result<(PathBuf, DateTime<Local>)> {
    let file_name = source
        .file_name()
        .ok_or_else(|| anyhow!("无法获取文件名: {}", source.display()))?;

    let metadata =
        fs::metadata(source).with_context(|| format!("无法读取文件信息: {}", source.display()))?;

    let datetime: DateTime<Local> = metadata.modified().context("无法读取文件修改时间")?.into();

    Ok((PathBuf::from(file_name), datetime))
}

fn build_dest_dir(dest: &Path, datetime: DateTime<Local>) -> Result<PathBuf> {
    let dest_dir = dest.join(datetime.format("%Y/%m-%d").to_string());

    if !dest_dir.exists() {
        fs::create_dir_all(&dest_dir)
            .with_context(|| format!("无法创建目录: {}", dest_dir.display()))?;
    }

    Ok(dest_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};
    use tempfile::tempdir;

    #[test]
    fn skips_and_accepts_expected_extensions() {
        assert!(!should_skip(Path::new("a.JPG")));
        assert!(should_skip(Path::new("note.txt")));
        assert!(should_skip(Path::new("Thumbs.db")));
    }

    #[test]
    fn copy_single_writes_to_dated_directory() -> Result<()> {
        let source_dir = tempdir()?;
        let dest_dir = tempdir()?;

        let source_file = source_dir.path().join("photo.jpg");
        fs::write(&source_file, b"hello")?;

        let modified: DateTime<Local> = fs::metadata(&source_file)?.modified()?.into();
        let expected_dir = dest_dir
            .path()
            .join(modified.format("%Y/%m-%d").to_string());

        let outcome = copy_single(&source_file, dest_dir.path())?;
        assert!(matches!(outcome, CopyOutcome::Copied));
        assert!(expected_dir.join("photo.jpg").exists());

        Ok(())
    }

    #[test]
    fn relocate_tracks_stats() -> Result<()> {
        let source_dir = tempdir()?;
        let dest_dir = tempdir()?;

        fs::write(source_dir.path().join("a.png"), b"ok")?;
        fs::write(source_dir.path().join("b.txt"), b"skip")?;

        let stats = relocate(source_dir.path(), dest_dir.path(), None)?;

        assert_eq!(stats.processed, 2);
        assert_eq!(stats.copied, 1);
        assert_eq!(stats.skipped, 1);
        assert_eq!(stats.errors, 0);

        Ok(())
    }
}
