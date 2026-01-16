# Chrono AVIF - 图片整理与 AVIF 转换工具

一个用 **纯 Rust** 编写的命令行工具，用于递归处理当前目录及其子目录下的图片文件，根据拍摄时间重命名并转换为 AVIF 格式。

## ✨ 纯 Rust 实现

本项目使用 **100% 纯 Rust** 实现，**无需任何系统依赖**（如 `dav1d`、`libavif` 等 C 库），可在 Windows、macOS、Linux 上直接编译运行，无需安装额外的系统库。

## 功能特性

- 🔍 **递归遍历**: 自动扫描程序运行时的当前工作目录及其所有子目录
- 📸 **支持格式**: `.jpg`, `.jpeg`, `.png`, `.tiff`
- ⏰ **智能时间获取**: 
  - 优先从 EXIF 元数据读取 `DateTimeOriginal`（拍摄时间）
  - 如果 EXIF 不存在，则使用文件系统创建时间
- 📝 **重命名规则**: `YYYY年MM月DD日 HH-mm-ss.avif`
- 🔄 **冲突处理**: 自动添加序号处理同名文件，如 `... 10-19-11(1).avif`
- 🎨 **AVIF 转换**: 使用纯 Rust 编码器（`ravif` + `rav1e`）
- ⚡ **并行处理**: 利用多核 CPU 并行处理，大幅提升转换速度
- 📁 **就地转换**: 转换后的文件保存在原文件所在目录，原图将被删除
- 🪟 **跨平台**: Windows、macOS、Linux 全平台支持，无需额外依赖

## 系统要求

- **Rust 工具链** (1.70+)
- **无需其他系统依赖** ✅

## 编译

### Windows

```powershell
# 确保已安装 Rust (https://www.rust-lang.org/tools/install)
# 然后直接编译，无需安装任何额外依赖
cargo build --release
```

### macOS / Linux

```bash
# 直接编译，无需安装系统库
cargo build --release
```

**重要**: 请务必使用 `--release` 模式编译，否则 AVIF 转换速度会非常慢。

## 使用方法

1. **进入包含图片的目录**（这是关键步骤）
2. 运行编译后的程序：

```bash
# Windows (PowerShell)
# 先切换到要处理的目录
cd C:\Users\YourName\Pictures
.\target\release\chrono_avif.exe

# macOS / Linux
# 先切换到要处理的目录
cd ~/Pictures
./target/release/chrono_avif

# 或使用 cargo run
cargo run --release
```

3. 程序会：
   - 处理**当前工作目录**及其**所有子目录**中的图片
   - 转换后的文件保存在原文件所在目录
   - 原图将被自动删除

**重要**: 程序只会处理运行时的**当前工作目录**及其子目录，不会处理其他位置的图片。

## 技术栈

- **walkdir**: 目录遍历
- **chrono**: 时间处理
- **rexif**: EXIF 解析（纯 Rust）
- **image**: 图片解码（JPEG/PNG/TIFF）
- **ravif**: AVIF 编码（纯 Rust，基于 rav1e）
- **rayon**: 并行计算
- **anyhow**: 错误处理

## 编码参数

- **Speed**: 6（平衡编码速度和质量，范围 0-10）
- **Quality**: 80（高质量，范围 0-100）

这些参数可以在 `src/main.rs` 的 `convert_to_avif` 函数中调整。

## Windows 使用说明

### 优势

- ✅ **无需安装系统库**: 不需要 `dav1d`、`libavif` 等 C 库
- ✅ **直接编译运行**: 只需 Rust 工具链即可
- ✅ **可分发二进制**: 编译后的 `.exe` 文件可以独立运行，无需额外 DLL

### 编译步骤

1. 安装 Rust: 访问 https://www.rust-lang.org/tools/install 下载安装
2. 打开 PowerShell 或 CMD，进入项目目录
3. 运行编译命令：

```powershell
cargo build --release
```

4. 运行程序：

```powershell
.\target\release\chrono_avif.exe
```

## 注意事项

- ⚠️ **重要**: 转换成功后，**原图将被删除**，请确保已备份重要图片
- 转换后的 AVIF 文件保存在原文件所在目录
- 如果目录中已存在同名文件，会自动添加序号（如 `(1)`, `(2)` 等）
- 处理大量图片时，建议使用 `--release` 模式编译以获得最佳性能
- AVIF 编码速度比 JPEG 慢，但压缩率更高，质量更好
- 生成的 AVIF 文件兼容性良好，可在现代浏览器和图片查看器中正常显示
- 建议先在小范围测试，确认转换效果符合预期后再批量处理

## 性能提示

- 使用 `--release` 模式编译可提升 10-100 倍性能
- 并行处理会自动利用所有 CPU 核心
- 对于大量图片，建议分批处理或使用 SSD 存储
