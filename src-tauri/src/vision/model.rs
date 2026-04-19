//! ONNX 推理工具函数
//!
//! 提供图像预处理和 embedding 计算的底层函数

use std::time::Instant;

use image::{imageops::FilterType, DynamicImage, GenericImageView};

use super::download::ModelManifest;

/// 图像预处理：解码 → 缩放裁切 → 归一化 → CHW 布局
///
/// 返回 (1, 3, size, size) 形状的 f32 向量
pub fn preprocess(image_bytes: &[u8], manifest: &ModelManifest) -> Result<Vec<f32>, String> {
    let t0 = Instant::now();

    // 1. 解码图片（对大图限制解码分辨率）
    let image = decode_with_size_limit(image_bytes, manifest.input_size as u32 * 4)?;
    let decode_us = t0.elapsed().as_micros();

    // 2. 快速缩放 + 中心裁切
    let t1 = Instant::now();
    let target = manifest.input_size as u32;
    let rgb = fast_crop_resize(&image, target);
    let resize_us = t1.elapsed().as_micros();

    // 3. 归一化 + CHW 转换
    let t2 = Instant::now();
    let size = manifest.input_size;
    let pixels = rgb.as_raw();
    let pixel_count = size * size;
    let mut chw = vec![0_f32; 3 * pixel_count];

    for i in 0..pixel_count {
        let base = i * 3;
        let r = (pixels[base] as f32 / 255.0 - manifest.mean[0]) / manifest.std[0];
        let g = (pixels[base + 1] as f32 / 255.0 - manifest.mean[1]) / manifest.std[1];
        let b = (pixels[base + 2] as f32 / 255.0 - manifest.mean[2]) / manifest.std[2];

        chw[i] = r;
        chw[pixel_count + i] = g;
        chw[2 * pixel_count + i] = b;
    }
    let norm_us = t2.elapsed().as_micros();

    println!(
        "[Vision] Preprocess: decode={}us, resize={}us, normalize={}us, total={}us (input {}x{})",
        decode_us, resize_us, norm_us, t0.elapsed().as_micros(),
        image.width(), image.height(),
    );

    Ok(chw)
}

/// 解码图片，如果尺寸远大于需要则快速缩小
fn decode_with_size_limit(image_bytes: &[u8], max_size: u32) -> Result<DynamicImage, String> {
    let image = image::load_from_memory(image_bytes).map_err(|e| e.to_string())?;
    let (w, h) = image.dimensions();

    if w <= max_size && h <= max_size {
        return Ok(image);
    }

    // 大图快速缩到 max_size 范围内（Nearest 几乎零开销）
    let scale = max_size as f32 / w.max(h) as f32;
    let new_w = (w as f32 * scale).round().max(1.0) as u32;
    let new_h = (h as f32 * scale).round().max(1.0) as u32;
    Ok(image.resize_exact(new_w, new_h, FilterType::Nearest))
}

/// 快速缩放 + 中心裁切
///
/// 策略：
/// - 如果原图远大于目标（>4x），先用 Nearest 粗缩到 2x 目标大小，再用 Triangle 精缩
/// - 否则直接用 Triangle（双线性，速度和质量均衡）
fn fast_crop_resize(image: &DynamicImage, target: u32) -> image::RgbImage {
    let (width, height) = image.dimensions();
    let short = width.min(height).max(1);

    // 先中心裁切成正方形（避免缩放非正方形大图）
    let crop_x = (width.saturating_sub(short)) / 2;
    let crop_y = (height.saturating_sub(short)) / 2;
    let cropped = image.crop_imm(crop_x, crop_y, short, short);

    if short > target * 4 {
        // 大图：先 Nearest 粗缩到 2x，再 Triangle 精缩
        let intermediate = target * 2;
        let rough = cropped.resize_exact(intermediate, intermediate, FilterType::Nearest);
        rough.resize_exact(target, target, FilterType::Triangle).to_rgb8()
    } else if short > target {
        // 中等图：直接 Triangle
        cropped.resize_exact(target, target, FilterType::Triangle).to_rgb8()
    } else {
        // 小图：直接用（可能需要放大）
        cropped.resize_exact(target, target, FilterType::Triangle).to_rgb8()
    }
}

/// L2 归一化
pub fn l2_normalize(mut vector: Vec<f32>) -> Vec<f32> {
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();

    if norm > 0.0 {
        for value in &mut vector {
            *value /= norm;
        }
    }

    vector
}
