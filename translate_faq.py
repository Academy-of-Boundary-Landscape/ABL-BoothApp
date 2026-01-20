import os
import glob
from pathlib import Path
from openai import OpenAI

# ================= 配置区域 =================
# 1. 填入你的 DeepSeek API Key
API_KEY="sk-9ec584e5691d4610a09ded98d5a33824"

# 2. 配置路径
SOURCE_DIR = "docs/faq"      # 中文源文件目录
TARGET_DIR = "docs/en/faq"   # 英文输出目录

# 3. DeepSeek 配置
BASE_URL = "https://api.deepseek.com"
MODEL_NAME = "deepseek-chat" # V3 模型性价比极高，适合翻译
# ===========================================

client = OpenAI(api_key=API_KEY, base_url=BASE_URL)

def get_system_prompt():
    """
    针对 VitePress Markdown 的专用翻译指令
    """
    return """
You are a professional technical translator and localization expert. 
Your task is to translate Markdown files from Simplified Chinese to English for a software documentation site.

RULES:
1. **Frontmatter**: Preserve the YAML Frontmatter (content between ---). 
   - Do NOT translate keys (e.g., `layout:`, `date:`).
   - ONLY translate values if they are human-readable text (e.g., `title:`, `description:`).
2. **VitePress Containers**: Do NOT translate or break custom containers like `:::tip`, `:::warning`, `:::info`. Translate the content inside them.
3. **Code**: Do NOT translate code blocks, file paths, variable names, or URLs.
4. **Links**: Keep internal links `[text](/path/to)` intact. Only translate the `[text]` part.
5. **Tone**: Professional, concise, and friendly (American English).
6. **Images**: Do not translate image paths.

Example Input:
---
title: 常见问题
---
:::tip 提示
请确保网络连接正常。
:::

Example Output:
---
title: FAQ
---
:::tip Tip
Please ensure your network connection is normal.
:::
    """

def translate_content(content, filename):
    """
    调用大模型进行翻译
    """
    print(f"🔄正在翻译: {filename} ...")
    
    try:
        response = client.chat.completions.create(
            model=MODEL_NAME,
            messages=[
                {"role": "system", "content": get_system_prompt()},
                {"role": "user", "content": content},
            ],
            temperature=0.1, # 低温度保证翻译准确性
            stream=False
        )
        return response.choices[0].message.content
    except Exception as e:
        print(f"❌ 翻译出错 {filename}: {str(e)}")
        return None

def main():
    # 1. 确保目标目录存在
    Path(TARGET_DIR).mkdir(parents=True, exist_ok=True)

    # 2. 获取源目录下所有 .md 文件
    files = glob.glob(os.path.join(SOURCE_DIR, "*.md"))
    
    if not files:
        print(f"⚠️ 在 {SOURCE_DIR} 下没有找到 .md 文件")
        return

    print(f"🚀 开始任务，共找到 {len(files)} 个文件")

    # 3. 循环处理
    for file_path in files:
        file_path = Path(file_path)
        filename = file_path.name
        
        # 读取原始内容
        with open(file_path, 'r', encoding='utf-8') as f:
            content = f.read()

        # 调用翻译
        translated_content = translate_content(content, filename)

        if translated_content:
            # 写入新文件
            target_path = os.path.join(TARGET_DIR, filename)
            with open(target_path, 'w', encoding='utf-8') as f:
                f.write(translated_content)
            print(f"✅ 已保存: {target_path}")

    print("\n🎉 全部翻译完成！请人工检查一遍生成的文档。")

if __name__ == "__main__":
    if API_KEY.startswith("sk-xxx"):
        print("❌ 请先在脚本中配置你的 DeepSeek API Key！")
    else:
        main()