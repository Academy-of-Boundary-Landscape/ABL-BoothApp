use std::env;
use std::path::PathBuf;

fn main() {
    // 1. 获取当前项目根目录 (src-tauri 目录)
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_path = PathBuf::from(manifest_dir);

    // 2. 拼接出前端 dist 的绝对路径（与 tauri.conf.json 的 frontendDist 一致）
    let dist_path = manifest_path.join("..").join("frontend").join("dist");

    // 3. 检查一下路径是否存在，给个编译时提示
    if !dist_path.exists() {
        println!("cargo:warning=⚠️⚠️⚠️ 前端 dist 目录不存在: {:?}", dist_path);
        println!("cargo:warning=请先执行 'npm run build' 并将 dist 复制到 src-tauri/dist");
    } else {
        println!("cargo:warning=✅ 成功锁定前端资源路径: {:?}", dist_path);
    }

    // 4. 将绝对路径设置为环境变量，供 RustEmbed 使用
    // 注意：Windows 路径的反斜杠可能会有问题，转成字符串处理
    println!(
        "cargo:rustc-env=FRONTEND_DIST_ABS_PATH={}",
        dist_path.display()
    );

    // 5. 告诉 Cargo，如果 dist 文件夹变了，需要重新编译
    println!("cargo:rerun-if-changed=../frontend/dist");
    tauri_build::build();
}
