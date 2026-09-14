/**
 * 判断文件是否应作为可编辑文本打开的小型纯函数。权威的二进制判断位于
 * `fs-gateway.cjs`（对前导字节做 NUL 探测）；这些前端辅助函数在读取往返之前
 * 提供基于扩展名的快速拦截，并在测试中镜像该探测逻辑。
 */

/** 永远不应作为纯文本编辑的扩展名。 */
export const NON_EDITABLE_EXTENSIONS = new Set([
  // 图片
  "png", "jpg", "jpeg", "gif", "webp", "svg", "ico", "bmp", "tiff", "avif",
  // 字体
  "woff", "woff2", "ttf", "otf", "eot",
  // 音频 / 视频
  "mp3", "mp4", "wav", "ogg", "avi", "mov", "mkv", "webm", "flac",
  // 压缩包
  "zip", "tar", "gz", "rar", "7z", "bz2", "xz",
  // 文档
  "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
  // 二进制
  "exe", "dll", "so", "dylib", "bin", "class", "o", "a", "wasm", "node",
]);

/** 返回文件名的小写扩展名，不含前导点。 */
export function extensionOf(fileName: string): string {
  const index = fileName.lastIndexOf(".");
  if (index <= 0 || index === fileName.length - 1) {
    return "";
  }
  return fileName.slice(index + 1).toLowerCase();
}

export function isBinaryExtension(fileName: string): boolean {
  return NON_EDITABLE_EXTENSIONS.has(extensionOf(fileName));
}

/** 在前导切片中检测 NUL 字节，镜像主进程的探测逻辑。 */
export function containsNullByte(content: string, limit = 8192): boolean {
  const end = Math.min(content.length, limit);
  for (let index = 0; index < end; index += 1) {
    if (content.charCodeAt(index) === 0) {
      return true;
    }
  }
  return false;
}
