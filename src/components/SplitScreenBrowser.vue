<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { Webview } from '@tauri-apps/api/webview';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { LogicalPosition, LogicalSize } from '@tauri-apps/api/dpi';

interface BrowserPanel {
  id: number;
  url: string;
  webviewLabel: string;
  isActive: boolean;
}

const panels = ref<BrowserPanel[]>([
  { id: 1, url: 'https://www.bing.com', webviewLabel: 'webview-1', isActive: false },
  { id: 2, url: 'https://www.baidu.com', webviewLabel: 'webview-2', isActive: false },
  { id: 3, url: 'https://www.google.com', webviewLabel: 'webview-3', isActive: false },
  { id: 4, url: 'https://github.com', webviewLabel: 'webview-4', isActive: false },
]);

const webviews = ref<Map<string, Webview>>(new Map());
const containerRefs = ref<Map<number, HTMLElement>>(new Map());

// 初始化 Webview
const initWebview = async (panel: BrowserPanel, container: HTMLElement) => {
  try {
    // 如果已存在，先移除
    if (webviews.value.has(panel.webviewLabel)) {
      const existingWebview = webviews.value.get(panel.webviewLabel);
      if (existingWebview) {
        await existingWebview.close();
      }
    }

    const rect = container.getBoundingClientRect();
    const window = await getCurrentWindow();

    // 创建新的 Webview
    const webview = new Webview(window, panel.webviewLabel, {
      url: panel.url,
      x: Math.round(rect.x),
      y: Math.round(rect.y + 64), // 加上导航栏高度
      width: Math.round(rect.width),
      height: Math.round(rect.height - 64),
    });

    // 监听 webview 创建成功事件
    webview.once('tauri://created', () => {
      console.log(`Webview ${panel.id} 创建成功`);
      panel.isActive = true;
    });

    // 监听 webview 创建失败事件
    webview.once('tauri://error', (e) => {
      console.error(`Webview ${panel.id} 创建失败:`, e);
    });

    webviews.value.set(panel.webviewLabel, webview);
  } catch (error) {
    console.error(`创建 Webview ${panel.id} 失败:`, error);
  }
};

// 处理 URL 提交
const handleUrlSubmit = async (panel: BrowserPanel) => {
  let url = panel.url.trim();
  if (!url) return;

  // 自动添加协议前缀
  if (!url.startsWith('http://') && !url.startsWith('https://')) {
    url = 'https://' + url;
    panel.url = url;
  }

  const container = containerRefs.value.get(panel.id);
  if (container) {
    await initWebview(panel, container);
  }
};

// 刷新面板
const refreshPanel = async (panel: BrowserPanel) => {
  const webview = webviews.value.get(panel.webviewLabel);
  if (webview) {
    // 关闭并重新创建 webview 来实现刷新
    await webview.close();
    const container = containerRefs.value.get(panel.id);
    if (container) {
      await initWebview(panel, container);
    }
  }
};

// 清空面板
const clearPanel = async (panel: BrowserPanel) => {
  const webview = webviews.value.get(panel.webviewLabel);
  if (webview) {
    await webview.close();
    webviews.value.delete(panel.webviewLabel);
  }
  panel.url = '';
  panel.isActive = false;
};

// 初始化所有 Webview
const initAllWebviews = async () => {
  // 等待 DOM 更新
  setTimeout(async () => {
    for (const panel of panels.value) {
      const container = containerRefs.value.get(panel.id);
      if (container && panel.url) {
        await initWebview(panel, container);
      }
    }
  }, 100);
};

// 清理所有 Webview
const cleanupAllWebviews = async () => {
  for (const [label, webview] of webviews.value.entries()) {
    try {
      await webview.close();
    } catch (error) {
      console.error(`关闭 Webview ${label} 失败:`, error);
    }
  }
  webviews.value.clear();
};

// 更新 Webview 位置和大小
const updateWebviewPositions = async () => {
  for (const panel of panels.value) {
    const webview = webviews.value.get(panel.webviewLabel);
    const container = containerRefs.value.get(panel.id);

    if (webview && container) {
      const rect = container.getBoundingClientRect();
      await webview.setPosition(
        new LogicalPosition(Math.round(rect.x), Math.round(rect.y + 64))
      );
      await webview.setSize(
        new LogicalSize(Math.round(rect.width), Math.round(rect.height - 64))
      );
    }
  }
};

onMounted(() => {
  initAllWebviews();

  // 监听窗口大小变化
  window.addEventListener('resize', updateWebviewPositions);
});

onUnmounted(() => {
  cleanupAllWebviews();
  window.removeEventListener('resize', updateWebviewPositions);
});
</script>

<template>
  <div class="split-screen-browser">
    <div class="browser-header">
      <h2 class="browser-title">🖥️ 四宫格浏览器</h2>
      <p class="browser-subtitle">基于 Tauri Webview，无限制浏览任何网站</p>
    </div>

    <div class="grid-container">
      <div
        v-for="panel in panels"
        :key="panel.id"
        class="browser-panel"
        :ref="(el) => { if (el) containerRefs.set(panel.id, el as HTMLElement) }"
      >
        <!-- 地址栏 -->
        <div class="address-bar">
          <div class="panel-number">#{{ panel.id }}</div>
          <input
            type="text"
            v-model="panel.url"
            @keyup.enter="handleUrlSubmit(panel)"
            placeholder="输入网址，按回车访问..."
            class="url-input"
          />
          <button
            @click="handleUrlSubmit(panel)"
            class="btn-go"
            :disabled="!panel.url.trim()"
          >
            访问
          </button>
          <button
            v-if="panel.isActive"
            @click="refreshPanel(panel)"
            class="btn-icon"
            title="刷新"
          >
            🔄
          </button>
          <button
            v-if="panel.isActive"
            @click="clearPanel(panel)"
            class="btn-icon"
            title="清空"
          >
            🗑️
          </button>
        </div>

        <!-- Webview 容器 -->
        <div class="webview-container">
          <div v-if="!panel.isActive" class="empty-state">
            <div class="empty-icon">🌐</div>
            <p>输入网址开始浏览</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.split-screen-browser {
  height: calc(100vh - 64px);
  display: flex;
  flex-direction: column;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}

.browser-header {
  padding: 16px 24px;
  background: rgba(255, 255, 255, 0.95);
  backdrop-filter: blur(10px);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
}

.browser-title {
  margin: 0;
  font-size: 24px;
  font-weight: 700;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.browser-subtitle {
  margin: 4px 0 0;
  font-size: 14px;
  color: #666;
}

.grid-container {
  flex: 1;
  display: grid;
  grid-template-columns: 1fr 1fr;
  grid-template-rows: 1fr 1fr;
  gap: 12px;
  padding: 12px;
  overflow: hidden;
}

.browser-panel {
  background: #fff;
  border-radius: 12px;
  overflow: hidden;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1);
  display: flex;
  flex-direction: column;
  transition: all 0.3s ease;
}

.browser-panel:hover {
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.15);
  transform: translateY(-2px);
}

.address-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  background: #f8f9fa;
  border-bottom: 1px solid #e9ecef;
}

.panel-number {
  font-weight: 700;
  color: #667eea;
  font-size: 14px;
  min-width: 28px;
}

.url-input {
  flex: 1;
  padding: 8px 12px;
  border: 1px solid #dee2e6;
  border-radius: 6px;
  font-size: 14px;
  transition: all 0.2s;
  background: #fff;
}

.url-input:focus {
  outline: none;
  border-color: #667eea;
  box-shadow: 0 0 0 3px rgba(102, 126, 234, 0.1);
}

.btn-go {
  padding: 8px 16px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: #fff;
  border: none;
  border-radius: 6px;
  font-size: 14px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-go:hover:not(:disabled) {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(102, 126, 234, 0.3);
}

.btn-go:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-icon {
  padding: 6px 10px;
  background: transparent;
  border: 1px solid #dee2e6;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
  font-size: 16px;
}

.btn-icon:hover {
  background: #e9ecef;
  border-color: #adb5bd;
}

.webview-container {
  flex: 1;
  position: relative;
  background: #fff;
  overflow: hidden;
}

.empty-state {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, #f8f9fa 0%, #e9ecef 100%);
  gap: 12px;
}

.empty-icon {
  font-size: 48px;
  opacity: 0.5;
}

.empty-state p {
  color: #999;
  font-size: 14px;
  margin: 0;
}
</style>
