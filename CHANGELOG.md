# Changelog

## 3.0.1

- 全新应用图标：蓝紫渐变 squircle + 白色 git 分支字形，16–1024px 全尺寸
- 图标改为脚本生成（`scripts/make-icon.py`），便于后续调整
- CI 并发构建时自动取消旧任务

## 3.0.0

- 重写为 **Tauri 2 + Vue 3**：原生窗口 + HTML/CSS 界面，去掉本地 HTTP 服务与端口依赖
- 新界面：极光背景、卡片光晕、置顶卡片旋转流光边框、状态点呼吸、按钮高光
- 卡片 / 列表双视图，搜索、全部 / 有改动 / 干净 筛选
- 置顶（收藏）：改 projects.json 的 pinned，排到最前
- 拉取：4 路并发 + 「拉取中 x/y」进度，拉取期间暂停轮询
- 添加项目支持原生多选文件夹
- 修复：置顶命令自锁死（点了没反应）、命令返回结构与前端不一致
- 打包：`scripts/make-app.sh` / `scripts/make-dmg.sh`，GitHub Actions 自动构建

## 2.0.0

- Rust + egui 重写，去掉 node 运行时与 47823 端口
- 代码、DMG 缩小到 4.6 MB / 2.5 MB

## 1.5.0

- node + server.js + 浏览器打开 localhost（旧版，已废弃）
