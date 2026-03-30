# Tauri + Vue3 + TypeScript 桌面应用

这是一个基于 Tauri、Vue3 和 TypeScript 构建的桌面应用程序。

## 技术栈

- **Tauri** - 构建轻量级桌面应用的框架
- **Vue 3** - 渐进式 JavaScript 框架
- **TypeScript** - 类型安全的 JavaScript 超集
- **Vite** - 下一代前端构建工具

## 开发环境要求

### 本地开发（浏览器预览）
- Node.js 16+
- Yarn 或 npm

### 构建桌面应用
- 以上所有
- Rust 环境
- Visual Studio Build Tools (Windows) / Xcode (macOS)

## 快速开始

### 1. 安装依赖

```bash
yarn install
```

### 2. 启动开发服务器（浏览器预览）

```bash
yarn dev
```

应用将在 http://localhost:1420 运行

### 3. 构建桌面应用

#### 方法一：本地构建（需要完整环境）

Windows 需要安装 [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)，选择"使用 C++ 的桌面开发"工作负载。

```bash
yarn tauri dev      # 开发模式
yarn tauri build    # 生产构建
```

#### 方法二：GitHub Actions 自动构建（推荐）

1. 将代码推送到 GitHub 仓库
2. GitHub Actions 会自动构建 Windows、macOS 和 Linux 版本
3. 在 Releases 页面下载安装包

## 项目结构

```
tauri/
├── src/                    # Vue3 前端代码
│   ├── App.vue            # 主组件
│   ├── main.ts            # 入口文件
│   └── styles.css         # 全局样式
├── src-tauri/             # Tauri Rust 后端代码
│   ├── src/
│   │   ├── main.rs        # 程序入口
│   │   └── lib.rs         # 后端命令
│   └── tauri.conf.json    # Tauri 配置
├── .github/workflows/     # GitHub Actions 配置
│   └── build.yml          # 自动构建工作流
└── package.json           # 项目依赖
```

## 添加后端命令

在 `src-tauri/src/lib.rs` 中添加：

```rust
#[tauri::command]
fn my_command(name: &str) -> String {
    format!("Hello, {}!", name)
}
```

在前端调用：

```typescript
import { invoke } from "@tauri-apps/api/core";

const result = await invoke("my_command", { name: "World" });
```

## 脚本命令

| 命令 | 说明 |
|------|------|
| `yarn dev` | 启动前端开发服务器 |
| `yarn build` | 构建前端生产版本 |
| `yarn tauri dev` | 启动桌面应用开发模式 |
| `yarn tauri build` | 构建桌面应用安装包 |

## 许可证

MIT
