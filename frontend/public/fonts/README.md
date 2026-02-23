# 本地图字体放置说明

项目当前使用落霞孤鹜体（LXGW WenKai）作为中文正文与等宽字体，需保留以下文件并保持文件名一致：

- `LXGWWenKai-Regular.ttf`（正文 Regular/400）
- `LXGWWenKai-Medium.ttf`（正文 Medium/600 用作按钮/标题）
- `LXGWWenKaiMono-Regular.ttf`（等宽 Regular/400）

如果你想更换为其他字重或字体：

1. 将新的 `.ttf` 复制到本目录并更新文件名。
2. 在 `frontend/app/globals.css` 的 `@font-face` 部分同步修改 `src` 路径与 `font-weight`。
3. 若字体包含授权文件（例如 `OFL.txt`），请同时放置在本目录。
