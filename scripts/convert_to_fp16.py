"""
Convert FP32 ONNX vision models to FP16, validate accuracy vs original.

Converts:
  - convnextv2_pico_fp32_tract.onnx  -> convnextv2_pico_fp16_tract.onnx
  - dinov2_small_fp32_tract.onnx     -> dinov2_small_fp16_tract.onnx

Intentionally skipped:
  - mobileclip_s0   (manifest.json flags all quantized variants as broken)

Usage:
  pip install onnx onnxconverter-common onnxruntime numpy pillow
  python scripts/convert_to_fp16.py

Outputs land in model-releases/release_fp16/ so you can inspect before
overwriting the release/ folder.
"""

import hashlib
import json
import sys
from pathlib import Path

import numpy as np
import onnx
import onnxruntime as ort
from onnxconverter_common import float16
from PIL import Image


ROOT = Path(__file__).resolve().parent.parent
SRC_DIR = ROOT / "model-releases" / "release-v1"
OUT_DIR = ROOT / "model-releases" / "release-v1-fp16-regen"


# (source file, output file, input_size, mean, std)
TARGETS = [
    (
        "convnextv2_pico_fp32.onnx",
        "convnextv2_pico_fp16.onnx",
        224,
        [0.485, 0.456, 0.406],
        [0.229, 0.224, 0.225],
    ),
    (
        "dinov2_small_fp32.onnx",
        "dinov2_small_fp16.onnx",
        224,
        [0.485, 0.456, 0.406],
        [0.229, 0.224, 0.225],
    ),
]


def preprocess(img: Image.Image, size: int, mean, std) -> np.ndarray:
    img = img.convert("RGB").resize((size, size), Image.BICUBIC)
    arr = np.asarray(img, dtype=np.float32) / 255.0
    arr = (arr - np.array(mean, dtype=np.float32)) / np.array(std, dtype=np.float32)
    arr = arr.transpose(2, 0, 1)[None, ...]  # -> [1, 3, H, W]
    return np.ascontiguousarray(arr)


def make_test_batch(size: int, mean, std, n: int = 8) -> np.ndarray:
    """Generate deterministic synthetic test images (no external files needed)."""
    rng = np.random.default_rng(42)
    batch = []
    for i in range(n):
        raw = rng.integers(0, 256, size=(size, size, 3), dtype=np.uint8)
        img = Image.fromarray(raw)
        batch.append(preprocess(img, size, mean, std))
    return np.concatenate(batch, axis=0)


def l2_normalize(x: np.ndarray) -> np.ndarray:
    n = np.linalg.norm(x, axis=-1, keepdims=True)
    return x / np.maximum(n, 1e-12)


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def convert_one(src_name: str, dst_name: str, size: int, mean, std) -> dict:
    src = SRC_DIR / src_name
    dst = OUT_DIR / dst_name

    if not src.exists():
        print(f"  [SKIP] {src_name} not found at {src}")
        return {"status": "missing"}

    print(f"\n=== {src_name} -> {dst_name} ===")

    # 1) Load + convert
    print("  [1/4] loading FP32 model...")
    model_fp32 = onnx.load(str(src))

    print("  [2/4] converting to FP16...")
    # keep_io_types=True keeps inputs/outputs as FP32 so the Rust side
    # (which always feeds f32 and reads f32) doesn't need to change.
    model_fp16 = float16.convert_float_to_float16(
        model_fp32,
        keep_io_types=True,
        disable_shape_infer=False,
    )

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    onnx.save(model_fp16, str(dst))

    src_mb = src.stat().st_size / (1024 * 1024)
    dst_mb = dst.stat().st_size / (1024 * 1024)
    print(f"         size: {src_mb:.1f} MB -> {dst_mb:.1f} MB  ({dst_mb/src_mb*100:.0f}%)")

    # 3) Numerical validation
    print("  [3/4] validating embeddings (cosine similarity FP32 vs FP16)...")
    batch = make_test_batch(size, mean, std, n=8)

    sess_fp32 = ort.InferenceSession(str(src), providers=["CPUExecutionProvider"])
    sess_fp16 = ort.InferenceSession(str(dst), providers=["CPUExecutionProvider"])

    in_name_fp32 = sess_fp32.get_inputs()[0].name
    in_name_fp16 = sess_fp16.get_inputs()[0].name

    emb_fp32 = sess_fp32.run(None, {in_name_fp32: batch})[0]
    emb_fp16 = sess_fp16.run(None, {in_name_fp16: batch})[0]

    # L2-normalize (registry marks these as normalize_output=true, but be safe)
    emb_fp32_n = l2_normalize(emb_fp32)
    emb_fp16_n = l2_normalize(emb_fp16)

    cos = np.sum(emb_fp32_n * emb_fp16_n, axis=-1)
    max_abs_diff = np.max(np.abs(emb_fp32 - emb_fp16))

    print(f"         per-sample cosine: min={cos.min():.6f} mean={cos.mean():.6f}")
    print(f"         max |FP32 - FP16|: {max_abs_diff:.6f}")

    verdict = "OK" if cos.min() > 0.999 else ("WARN" if cos.min() > 0.99 else "BROKEN")
    print(f"         verdict: {verdict}")

    # 4) SHA256
    print("  [4/4] computing SHA256...")
    digest = sha256_file(dst)
    print(f"         {digest}")

    return {
        "status": "ok",
        "src_mb": round(src_mb, 1),
        "dst_mb": round(dst_mb, 1),
        "cos_min": float(cos.min()),
        "cos_mean": float(cos.mean()),
        "max_abs_diff": float(max_abs_diff),
        "sha256": digest,
        "verdict": verdict,
    }


def main():
    if not SRC_DIR.exists():
        print(f"ERROR: source dir not found: {SRC_DIR}")
        sys.exit(1)

    print(f"Source:  {SRC_DIR}")
    print(f"Output:  {OUT_DIR}")

    results = {}
    for src_name, dst_name, size, mean, std in TARGETS:
        results[dst_name] = convert_one(src_name, dst_name, size, mean, std)

    # Write a report file
    report = OUT_DIR / "fp16_conversion_report.json"
    report.write_text(json.dumps(results, indent=2), encoding="utf-8")
    print(f"\nReport written to {report}")

    print("\n=== Summary ===")
    for name, r in results.items():
        if r.get("status") != "ok":
            print(f"  {name:45s} [{r.get('status')}]")
        else:
            print(
                f"  {name:45s} {r['verdict']:6s} "
                f"cos.min={r['cos_min']:.4f} "
                f"{r['src_mb']:.0f}MB -> {r['dst_mb']:.0f}MB"
            )


if __name__ == "__main__":
    main()
