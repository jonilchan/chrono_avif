use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use image::ImageReader;
use image::DynamicImage;
use rexif;
use rayon::prelude::*;
use ravif::{Encoder, Img, RGBA8};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use walkdir::WalkDir;

// 支持的图片扩展名
const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "tiff"];

fn main() -> Result<()> {
    // 获取当前工作目录（程序运行的目录）
    let current_dir = std::env::current_dir()
        .context("无法获取当前工作目录")?;
    
    println!("🚀 开始处理图片文件...");
    println!("📁 处理目录: {}", current_dir.display());
    println!("⚠️  注意：转换后的文件将保存在原文件夹，原图将被删除");
    println!("📂 将递归处理当前目录及其所有子目录\n");

    // 收集所有图片文件（只处理当前目录及其子目录）
    let image_files: Vec<PathBuf> = WalkDir::new(&current_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
                    .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    let total = image_files.len();
    println!("📸 找到 {} 个图片文件", total);

    if total == 0 {
        println!("⚠️  未找到任何图片文件");
        return Ok(());
    }

    // 用于统计进度
    let processed: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    let deleted: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));

    // 使用 rayon 并行处理所有图片
    let results: Vec<Result<()>> = image_files
        .par_iter()
        .map(|image_path| {
            process_image(image_path, &processed, &deleted, total)
        })
        .collect();

    // 检查是否有错误
    let errors: Vec<_> = results.into_iter().filter_map(|r| r.err()).collect();
    
    let deleted_count = *deleted.lock().unwrap();
    
    if !errors.is_empty() {
        eprintln!("\n❌ 处理过程中遇到 {} 个错误:", errors.len());
        for err in &errors {
            eprintln!("  - {}", err);
        }
    }

    println!("\n✅ 处理完成！");
    println!("   - 成功转换: {} 个文件", total - errors.len());
    println!("   - 已删除原图: {} 个文件", deleted_count);
    Ok(())
}

/// 处理单个图片文件
fn process_image(
    image_path: &Path,
    processed: &Arc<Mutex<usize>>,
    deleted: &Arc<Mutex<usize>>,
    total: usize,
) -> Result<()> {
    // 获取原文件所在目录
    let parent_dir = image_path.parent()
        .ok_or_else(|| anyhow::anyhow!("无法获取文件目录"))?;

    // 获取拍摄时间
    let datetime = get_image_datetime(image_path)
        .with_context(|| format!("无法获取图片时间: {}", image_path.display()))?;

    // 格式化时间为目标文件名格式：YYYY年MM月DD日 HH-mm-ss
    let formatted_time = datetime.format("%Y年%m月%d日 %H-%M-%S").to_string();
    
    // 生成基础文件名
    let base_filename = format!("{}.avif", formatted_time);
    
    // 处理文件名冲突（在原目录中检查）
    let final_filename = generate_unique_filename(
        parent_dir,
        &base_filename,
    )?;

    let output_path = parent_dir.join(&final_filename);

    // 读取并转换图片为 AVIF
    convert_to_avif(image_path, &output_path)
        .with_context(|| format!("转换失败: {} -> {}", image_path.display(), output_path.display()))?;

    // 删除原文件
    fs::remove_file(image_path)
        .with_context(|| format!("无法删除原文件: {}", image_path.display()))?;

    // 更新进度
    let mut count = processed.lock().unwrap();
    *count += 1;
    let mut del_count = deleted.lock().unwrap();
    *del_count += 1;
    
    println!(
        "[{}/{}] ✅ {} -> {} (已删除原图)",
        *count,
        total,
        image_path.file_name().unwrap_or_default().to_string_lossy(),
        final_filename
    );

    Ok(())
}

/// 获取图片的拍摄时间
/// 优先级：1. EXIF DateTimeOriginal  2. 文件系统创建时间
fn get_image_datetime(image_path: &Path) -> Result<DateTime<Local>> {
    // 尝试从 EXIF 读取 DateTimeOriginal
    if let Ok(datetime) = get_exif_datetime(image_path) {
        return Ok(datetime);
    }

    // 如果 EXIF 不存在，使用文件系统元数据
    let metadata = fs::metadata(image_path)
        .context("无法读取文件元数据")?;
    
    // 优先使用创建时间，如果没有则使用修改时间
    let system_time = metadata
        .created()
        .or_else(|_| metadata.modified())
        .context("无法获取文件时间")?;
    
    let datetime: DateTime<Local> = system_time.into();
    Ok(datetime)
}

/// 从 EXIF 元数据中读取 DateTimeOriginal
fn get_exif_datetime(image_path: &Path) -> Result<DateTime<Local>> {
    // 使用 rexif 读取 EXIF 数据
    let file_data = fs::read(image_path)
        .context("无法读取文件")?;
    
    let exif_data = rexif::parse_buffer(&file_data)
        .context("无法解析 EXIF 数据")?;

    // 查找 DateTimeOriginal 字段
    for entry in exif_data.entries {
        if entry.tag == rexif::ExifTag::DateTimeOriginal {
            // EXIF DateTimeOriginal 格式: "YYYY:MM:DD HH:MM:SS"
            let datetime_str = entry.value_more_readable;
            if let Ok(naive_dt) = NaiveDateTime::parse_from_str(&datetime_str, "%Y:%m:%d %H:%M:%S") {
                // 将 NaiveDateTime 转换为 Local DateTime
                return Ok(Local.from_local_datetime(&naive_dt)
                    .single()
                    .ok_or_else(|| anyhow::anyhow!("无效的时区转换"))?);
            }
        }
    }

    Err(anyhow::anyhow!("EXIF 中未找到 DateTimeOriginal"))
}

/// 生成唯一的文件名，处理冲突（检查目录中是否已存在同名文件）
fn generate_unique_filename(
    parent_dir: &Path,
    base_filename: &str,
) -> Result<String> {
    let base_path = parent_dir.join(base_filename);
    
    // 如果文件名不存在，直接返回
    if !base_path.exists() {
        return Ok(base_filename.to_string());
    }

    // 处理冲突：添加序号
    let (name_without_ext, _ext) = base_filename.rsplit_once('.').unwrap_or((base_filename, ""));
    let mut counter = 1;
    
    loop {
        let new_filename = format!("{}({}).avif", name_without_ext, counter);
        let new_path = parent_dir.join(&new_filename);
        
        if !new_path.exists() {
            return Ok(new_filename);
        }
        
        counter += 1;
        
        // 防止无限循环（理论上不会发生，但安全起见）
        if counter > 10000 {
            return Err(anyhow::anyhow!("文件名冲突过多，无法生成唯一文件名"));
        }
    }
}

/// 将图片转换为 AVIF 格式（使用纯 Rust 的 ravif 库）
fn convert_to_avif(input_path: &Path, output_path: &Path) -> Result<()> {
    // 使用 image 库读取图片
    let img: DynamicImage = ImageReader::open(input_path)
        .context("无法打开图片文件")?
        .decode()
        .context("无法解码图片")?;

    // 将图像转换为 RGBA8 格式（ravif 需要 RGBA）
    let rgba_img = img.to_rgba8();
    let (width, height) = rgba_img.dimensions();

    // 配置 AVIF 编码参数
    // speed: 6 (平衡编码速度和质量，范围 0-10，数字越大速度越快)
    // quality: 80 (高质量，范围 0-100)
    let encoder = Encoder::new()
        .with_quality(80.0)
        .with_speed(6);

    // 编码为 AVIF
    // ravif 需要 Img<&[RGBA8]> 格式
    // 将 &[u8] 转换为 &[RGBA8]
    let pixels_u8 = rgba_img.as_raw();
    let pixels_rgba: &[RGBA8] = unsafe {
        std::slice::from_raw_parts(
            pixels_u8.as_ptr() as *const RGBA8,
            pixels_u8.len() / 4,
        )
    };
    let img = Img::new(pixels_rgba, width as usize, height as usize);
    let encoded = encoder
        .encode_rgba(img)
        .context("AVIF 编码失败")?;

    // 保存到文件
    fs::write(output_path, encoded.avif_file)
        .context("无法写入输出文件")?;

    Ok(())
}
