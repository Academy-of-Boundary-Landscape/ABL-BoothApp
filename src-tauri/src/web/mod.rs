use axum::{
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use include_dir::{include_dir, Dir}; // 引入 include_dir
use std::env;

// ========================================================
// 核心：使用 include_dir! 宏
// 1. $CARGO_MANIFEST_DIR 确保是绝对路径
// 2. "dist" 指向 src-tauri/dist
// ========================================================
static FRONTEND_ASSETS: Dir = include_dir!("../frontend/dist");

pub async fn static_file_handler(uri: Uri, _upload_dir: std::path::PathBuf) -> Response {
    let path = uri.path().trim_start_matches('/');

    println!("--------------------------------------------------");
    println!(">>> 收到请求路径: '{}'", path);

    // 1. 打印当前环境诊断信息 (只在请求 index.html 或根路径时打印)
    if path.is_empty() || path == "index.html" {
        println!(">>> [环境诊断] 正在运行诊断...");

        match env::current_dir() {
            Ok(cwd) => println!(">>> [环境诊断] 手机当前工作目录: {:?}", cwd),
            Err(e) => println!(">>> [环境诊断] 无法获取工作目录: {}", e),
        }

        println!(">>> [资源清单] 开始检查 include_dir 嵌入的文件...");

        // 简单的递归计数函数，用于统计文件总数
        fn count_files(dir: &Dir) -> usize {
            let mut count = dir.files().count();
            for subdir in dir.dirs() {
                count += count_files(subdir);
            }
            count
        }

        let total_files = count_files(&FRONTEND_ASSETS);

        // 打印根目录下的部分文件作为验证
        for file in FRONTEND_ASSETS.files() {
            println!("   - (根目录) 已打包文件: {:?}", file.path());
        }

        if total_files == 0 {
            println!(">>> [严重警告] include_dir 列表为空！编译时可能指向了空文件夹！");
        } else {
            println!(
                ">>> [资源清单] 共找到 {} 个嵌入文件 (含子目录)。",
                total_files
            );
        }
        println!("--------------------------------------------------");
    }

    // ----------------------------------------------------
    // 2. 正常处理逻辑
    // ----------------------------------------------------

    // include_dir 的 get_file 需要准确的路径
    // 如果请求的是空路径，默认映射到 index.html (Axum通常会把 / 变成 empty path)
    let search_path = if path.is_empty() { "index.html" } else { path };

    // A. 尝试直接获取文件
    if let Some(file) = FRONTEND_ASSETS.get_file(search_path) {
        println!(">>> [成功] 返回静态资源: {}", search_path);
        let mime = mime_guess::from_path(search_path).first_or_octet_stream();
        return ([(header::CONTENT_TYPE, mime.as_ref())], file.contents()).into_response();
    }

    // B. SPA 回退逻辑 (如果找不到资源，返回 index.html)
    // 注意：如果是 API 请求或其他非页面资源，可能不应该回退，但在 SPA 中通常这样做
    if let Some(index) = FRONTEND_ASSETS.get_file("index.html") {
        println!(">>> [回退] 路径 '{}' 未找到，回退到 index.html", path);
        return ([(header::CONTENT_TYPE, "text/html")], index.contents()).into_response();
    }

    println!(
        ">>> [失败] 彻底放弃治疗。未找到 '{}' 且未找到 'index.html'",
        path
    );
    (StatusCode::NOT_FOUND, "404 Not Found").into_response()
}
