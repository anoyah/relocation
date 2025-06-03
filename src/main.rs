use std::{
    fs,
    io::{self},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Local};
use clap::Parser;

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
    let cli: Cli = Cli::parse();

    let source = cli.source.expect("原目录不能为空");
    let dest = cli.dest.expect("目标目录不能为空");
    let all_filepath = get_all_filepath(source);

    for filepath in all_filepath.clone() {
        // let file_create_time = get_file_create_time(filepath.clone());
        copy_file(filepath, dest.clone()).expect("复制文件发生错误");
        println!("复制文件成功");
    }
}

fn get_all_filepath(source: String) -> Vec<String> {
    let mut filepath: Vec<String> = Vec::new();

    let folder_path = PathBuf::from(source);

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

    let path = mkdir(dest.clone(), datetime).unwrap();

    let dest_filename = PathBuf::from(format!("{}/{}", path, file_name));

    fs::copy(source, dest_filename)?;
    Ok(0)
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
    let dt = datetime.format("%Y/%m/%d").to_string();

    // 使用年月日构建目录
    let dest_dir = format!("{}/{}", dest, dt);
    let dest_path = Path::new(&dest_dir);
    // 创建目录（如果不存在）
    if !dest_path.exists() {
        fs::create_dir_all(dest_path)?;
    }

    Ok(dest_dir)
}
