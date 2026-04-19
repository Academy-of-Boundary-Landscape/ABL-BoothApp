// src/utils/file.rs

use axum::extract::multipart::Field;
use std::path::PathBuf;
use tokio::fs;
use uuid::Uuid;

pub async fn save_upload_bytes(
    base_dir: &PathBuf,
    data: &[u8],
    original_file_name: Option<&str>,
    sub_folder: Option<&str>,
) -> Result<String, String> {
    let ext = original_file_name
        .and_then(|name| std::path::Path::new(name).extension())
        .and_then(|e| e.to_str())
        .unwrap_or("jpg");

    let new_filename = format!("{}.{}", Uuid::new_v4(), ext);

    let mut file_path = base_dir.clone();
    let mut relative_path_str = String::new();

    if let Some(folder) = sub_folder {
        file_path.push(folder);
        if !file_path.exists() {
            fs::create_dir_all(&file_path)
                .await
                .map_err(|e| e.to_string())?;
        }
        relative_path_str.push_str(folder);
        relative_path_str.push('/');
    }

    file_path.push(&new_filename);
    relative_path_str.push_str(&new_filename);

    fs::write(&file_path, data)
        .await
        .map_err(|e| format!("Write failed: {}", e))?;

    Ok(format!("/uploads/{}", relative_path_str))
}

/// 保存上传的文件
///
/// - `base_dir`: 物理根目录 (state.upload_dir)
/// - `field`: Axum 的 Multipart Field
/// - `sub_folder`: (可选) 子文件夹，比如 "events" 或 "products"，方便分类
///
/// 返回: Result<可直接访问的完整路径 (包含 /uploads/ 前缀), 错误信息>
pub async fn save_upload_file(
    base_dir: &PathBuf,
    field: Field<'_>,
    sub_folder: Option<&str>,
) -> Result<String, String> {
    let file_name = field.file_name().unwrap_or("unknown").to_string();
    let data = field.bytes().await.map_err(|e| e.to_string())?;
    save_upload_bytes(base_dir, &data, Some(&file_name), sub_folder).await
}

/// 删除文件
///
/// - `relative_path`: 完整路径，如 "uploads/products/xxx.jpg"
pub async fn delete_file(base_dir: &PathBuf, relative_path: &str) -> std::io::Result<()> {
    if relative_path.is_empty() {
        return Ok(());
    }

    // 防御性编程：防止路径回溯攻击 (../)
    if relative_path.contains("..") {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Invalid path",
        ));
    }

    // 去掉 "uploads/" 前缀（如果存在）
    let path_without_uploads = relative_path.trim_start_matches("uploads/");

    let file_path = base_dir.join(path_without_uploads);
    if file_path.exists() {
        fs::remove_file(file_path).await?;
    }
    Ok(())
}
