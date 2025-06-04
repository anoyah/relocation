use std::{
    fs,
    io::{self},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local};
use clap::Parser;
use env_logger::Builder;
use log::{LevelFilter, debug, error, info};

// 过滤文件
const IGNORE_FILENAME: [&str; 2] = ["Thumbs.db", ".DS_Store"];

// 允许的文件后缀
const SUFFIX: [&str; 21] = [
    "jpg", "mp4", "png", "gif", "bmp", "tiff", "tif", "webp", "svg", "ico", "mp3", "wav", "aac",
    "ogg", "flac", "m4a", "mov", "avi", "wmv", "mkv", "webm",
];

#[derive(Parser)]
#[command(version,about, long_about = None)]
struct Cli {
    /// 设置转移原目录
    #[arg(short, long)]
    source: Option<String>,
    /// 设置转移目标目录
    #[arg(short, long)]
    dest: Option<String>,
    // /// 打印帮助信息
    // help: Option<String>,
    // /// 打印版本号
    // version: Option<String>,
}

// `relo -s /path/to/source -d /path/to/dest`
fn main() {
    let mut builder = Builder::new();

    builder
        .filter_level(LevelFilter::Info) // 设置过滤级别
        // .format(|buf, record| writeln!(buf, "{}", record.args()))
        .init();

    let cli: Cli = Cli::parse();

    let source = cli.source.unwrap_or_default();
    if source.is_empty() {
        error!("目录不能为空");
        return;
    }

    let dest = cli.dest.unwrap_or_default();
    if dest.is_empty() {
        error!("目标目录不能为空");
        return;
    }

    let all_filepath = get_all_filepath(source);

    for filepath in all_filepath.clone() {
        copy_file(filepath, dest.clone()).expect("复制文件发生错误");
    }
}

fn get_all_filepath(source: String) -> Vec<String> {
    let mut filepath: Vec<String> = Vec::new();

    let folder_path: PathBuf = PathBuf::from(source);

    if !folder_path.exists() {
        println!("文件夹不存在: {}", folder_path.display());
        return filepath;
    }

    if !folder_path.is_dir() {
        println!("路径不是一个文件夹: {}", folder_path.display());
        return filepath;
    }

    for entry in fs::read_dir(folder_path).expect("无法读取文件夹") {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if path.is_file() {
                    filepath.push(path.display().to_string());
                }

                // 检查是否为子文件夹，如果是，则递归调用
                if path.is_dir() {
                    filepath.extend(get_all_filepath(path.display().to_string())); // 传递子文件夹路径
                }
            }
            Err(e) => {
                println!("读取文件时出错: {}", e);
            }
        }
    }

    return filepath;
}

// move_file save file to "year/month/day"
fn copy_file(source: String, dest: String) -> io::Result<u64> {
    let (file_name, datetime) = get_file_info(&source);

    if filter_file_name(&file_name) {
        return Ok(0);
    }

    let path: String = mkdir(dest.clone(), datetime).unwrap();
    let dest_filename: PathBuf = PathBuf::from(format!("{}/{}", path, file_name));

    if is_exist(&dest_filename) {
        println!("目标文件已存在: {}", dest_filename.to_string_lossy());
        return Ok(0);
    }

    fs::copy(source.clone(), dest_filename.clone())?;

    println!(
        "源文件:{} \n目标文件:{} \n\n",
        source,
        dest_filename.to_string_lossy()
    );

    Ok(0)
}

//
fn filter_file_name(file_name: &str) -> bool {
    for igfn in IGNORE_FILENAME {
        if file_name.eq(igfn) {
            return true;
        }
    }

    for suffix in SUFFIX {
        if file_name.ends_with(suffix) {
            return false;
        }
    }

    true
}

fn get_file_info(source: &String) -> (std::borrow::Cow<'_, str>, DateTime<Local>) {
    // 获取文件的信息
    let path = Path::new(source);
    let file_name = path.file_name().unwrap().to_string_lossy();

    let md = fs::metadata(path).unwrap();
    let datetime: DateTime<Local> = md.created().unwrap().into();
    (file_name, datetime)
}

fn mkdir(dest: String, datetime: DateTime<Local>) -> io::Result<String> {
    let dt = datetime.format("%Y/%m-%d").to_string();

    // 使用年月日构建目录
    let dest_dir = format!("{}/{}", dest, dt);
    let dest_path = Path::new(&dest_dir);
    // 创建目录（如果不存在）
    if !dest_path.exists() {
        fs::create_dir_all(dest_path)?;
    }

    Ok(dest_dir)
}

// is_exist 判断文件是否存在
fn is_exist(file_path: &PathBuf) -> bool {
    let path = Path::new(file_path.to_str().unwrap());
    path.exists()
}
