<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { Webview } from '@tauri-apps/api/webview';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { LogicalPosition, LogicalSize } from '@tauri-apps/api/dpi';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

interface BrowserPanel {
  id: number;
  url: string;
  initialUrl: string; // 保存初始 URL，用于刷新
  webviewLabel: string;
  isActive: boolean;
}

const panels = ref<BrowserPanel[]>([
  { id: 1, url: 'https://arca.rwadt.com/', initialUrl: 'https://arca.rwadt.com/', webviewLabel: 'webview-1', isActive: false },
  { id: 2, url: 'https://lnsu.rwadt.com/', initialUrl: 'https://lnsu.rwadt.com/', webviewLabel: 'webview-2', isActive: false },
  { id: 3, url: 'https://os.rwadt.com/', initialUrl: 'https://os.rwadt.com/', webviewLabel: 'webview-3', isActive: false },
  { id: 4, url: 'https://yvan.rwadt.com/', initialUrl: 'https://yvan.rwadt.com/', webviewLabel: 'webview-4', isActive: false },
]);

const webviews = ref<Map<string, Webview>>(new Map());
const containerRefs = ref<Map<number, HTMLElement>>(new Map());
const gridContainerRef = ref<HTMLElement | null>(null);

// 跟踪用户是否正在编辑地址栏
const editingPanels = ref<Set<number>>(new Set());

// 自动同步登录状态配置
const autoSyncEnabled = ref(true); // 是否启用自动同步
const autoSyncTriggered = ref<Set<number>>(new Set()); // 记录已触发自动同步的面板，避免重复触发

// 分隔条位置状态（百分比）
const horizontalSplit = ref(50); // 水平分隔条位置（左右分割）
const verticalSplit = ref(50); // 垂直分隔条位置（上下分割）

// 拖拽状态
const isDraggingHorizontal = ref(false);
const isDraggingVertical = ref(false);

// 拖拽起始位置
const dragStartX = ref(0);
const dragStartY = ref(0);
const dragStartHorizontalSplit = ref(50);
const dragStartVerticalSplit = ref(50);

// 开始拖拽水平分隔条
const startDragHorizontal = (e: MouseEvent) => {
  isDraggingHorizontal.value = true;
  dragStartX.value = e.clientX;
  dragStartHorizontalSplit.value = horizontalSplit.value;
  e.preventDefault();
};

// 开始拖拽垂直分隔条
const startDragVertical = (e: MouseEvent) => {
  isDraggingVertical.value = true;
  dragStartY.value = e.clientY;
  dragStartVerticalSplit.value = verticalSplit.value;
  e.preventDefault();
};

// 开始拖拽交叉点（同时调整水平和垂直）
const startDragCross = (e: MouseEvent) => {
  isDraggingHorizontal.value = true;
  isDraggingVertical.value = true;
  dragStartX.value = e.clientX;
  dragStartY.value = e.clientY;
  dragStartHorizontalSplit.value = horizontalSplit.value;
  dragStartVerticalSplit.value = verticalSplit.value;
  e.preventDefault();
};

// 处理拖拽移动
const handleMouseMove = (e: MouseEvent) => {
  if (!gridContainerRef.value) return;

  const rect = gridContainerRef.value.getBoundingClientRect();

  if (isDraggingHorizontal.value) {
    // 计算水平分隔条位置（左右分割）
    const deltaX = e.clientX - dragStartX.value;
    const deltaPercent = (deltaX / rect.width) * 100;
    const newPosition = dragStartHorizontalSplit.value + deltaPercent;
    horizontalSplit.value = Math.max(20, Math.min(80, newPosition));
  }

  if (isDraggingVertical.value) {
    // 计算垂直分隔条位置（上下分割）
    const deltaY = e.clientY - dragStartY.value;
    const deltaPercent = (deltaY / rect.height) * 100;
    const newPosition = dragStartVerticalSplit.value + deltaPercent;
    verticalSplit.value = Math.max(20, Math.min(80, newPosition));
  }

  // 如果有任何拖拽，更新 webview 位置
  if (isDraggingHorizontal.value || isDraggingVertical.value) {
    updateWebviewPositions();
  }
};

// 停止拖拽
const stopDrag = () => {
  isDraggingHorizontal.value = false;
  isDraggingVertical.value = false;
};

// 初始化 Webview - 使用 Rust 命令创建，支持拦截 target="_blank" 链接
const initWebview = async (panel: BrowserPanel, container: HTMLElement) => {
  try {
    const rect = container.getBoundingClientRect();

    // 如果已存在，先彻底清理
    const existingWebview = webviews.value.get(panel.webviewLabel);
    if (existingWebview) {
      try {
        // 先移除所有事件监听器
        const webviewWithCleanup = existingWebview as Webview & { _unlisteners?: (() => void)[] };
        if (webviewWithCleanup._unlisteners) {
          webviewWithCleanup._unlisteners.forEach(unlisten => unlisten());
        }
        await existingWebview.close();
      } catch (e) {
        console.warn(`关闭已存在的 Webview ${panel.id} 时出错:`, e);
      }
      webviews.value.delete(panel.webviewLabel);
      // 等待 Webview 完全关闭
      await new Promise(resolve => setTimeout(resolve, 200));
    }

    // 使用 Rust 命令创建 webview，它会自动注入拦截脚本
    const currentWindow = await getCurrentWindow();
    await invoke('create_webview_with_handler', {
      label: panel.webviewLabel,
      url: panel.url,
      initialUrl: panel.initialUrl,
      x: Math.round(rect.x),
      y: Math.round(rect.y + 64), // 加上导航栏高度
      width: Math.round(rect.width),
      height: Math.round(rect.height - 64),
    });

    console.log(`Webview ${panel.id} 创建成功:`, panel.url);
    panel.isActive = true;

    // 获取 webview 实例用于监听事件（复用 currentWindow）
    const webview = new Webview(currentWindow, panel.webviewLabel, {
      url: panel.initialUrl, // 使用初始 URL 保持一致
      x: Math.round(rect.x),
      y: Math.round(rect.y + 64),
      width: Math.round(rect.width),
      height: Math.round(rect.height - 64),
    });

    // 存储监听器取消函数，用于清理
    const unlisteners: (() => void)[] = [];

    // 监听 webview 创建成功事件
    webview.once('tauri://created', () => {
      console.log(`Webview ${panel.id} 事件监听已启动`);
    });

    // 监听导航事件，更新地址栏 URL（用户点击链接时触发）
    const navigateUnlisten = await webview.listen('tauri://navigate', (event: { payload: { url: string } }) => {
      const newUrl = event.payload.url;
      console.log(`Webview ${panel.id} 导航到:`, newUrl);
      panel.url = newUrl;
    });
    unlisteners.push(navigateUnlisten);

    // 监听页面加载完成事件
    const loadUnlisten = await webview.listen('tauri://load', (event: { payload: { url: string } }) => {
      const loadedUrl = event.payload.url;
      console.log(`Webview ${panel.id} 加载完成:`, loadedUrl);
      panel.url = loadedUrl;

      // 自动同步登录状态检测
      // 如果启用了自动同步，且当前面板未触发过同步，且URL与初始URL不同（可能是登录后跳转）
      if (autoSyncEnabled.value &&
          !autoSyncTriggered.value.has(panel.id) &&
          loadedUrl !== panel.initialUrl &&
          panel.isActive) {
        console.log(`Webview ${panel.id} 检测到URL变化，自动同步其他面板`);
        autoSyncTriggered.value.add(panel.id);
        // 延迟一下再刷新其他面板，确保登录状态已保存
        setTimeout(() => {
          refreshOtherPanels(panel);
        }, 1000);
      }
    });
    unlisteners.push(loadUnlisten);

    // 监听 URL 变化事件（某些情况下比 navigate 更可靠）
    const urlUnlisten = await webview.listen('tauri://url', (event: { payload: string }) => {
      const newUrl = event.payload;
      console.log(`Webview ${panel.id} URL 变化:`, newUrl);
      panel.url = newUrl;
    });
    unlisteners.push(urlUnlisten);

    // 定期轮询获取当前 URL（备用方案，确保地址栏始终显示正确 URL）
    const pollInterval = setInterval(async () => {
      // 如果用户正在编辑此面板的地址栏，跳过更新
      if (editingPanels.value.has(panel.id)) {
        return;
      }
      try {
        // 使用 Rust 命令获取当前 URL
        const currentUrl = await invoke<string>('get_webview_url', {
          label: panel.webviewLabel
        });
        if (currentUrl && currentUrl !== panel.url) {
          panel.url = currentUrl;
        }
      } catch (e) {
        // webview 可能已关闭，忽略错误
      }
    }, 500); // 每 500ms 检查一次

    // 将清理函数添加到 unlisteners
    unlisteners.push(() => clearInterval(pollInterval));

    // 将 webview 和清理函数一起存储
    const webviewWithCleanup = webview as Webview & { _unlisteners?: (() => void)[] };
    webviewWithCleanup._unlisteners = unlisteners;
    webviews.value.set(panel.webviewLabel, webviewWithCleanup);
  } catch (error) {
    console.error(`创建 Webview ${panel.id} 失败:`, error);
    panel.isActive = false;
  }
};

// 处理 URL 提交
const handleUrlSubmit = async (panel: BrowserPanel) => {
  let url = panel.url.trim();
  if (!url) return;

  // 自动添加协议前缀
  if (!url.startsWith('http://') && !url.startsWith('https://')) {
    url = 'https://' + url;
  }
  panel.url = url;
  // 更新初始 URL，用于后续刷新
  panel.initialUrl = url;

  const container = containerRefs.value.get(panel.id);
  if (container) {
    await initWebview(panel, container);
  }
};

// 刷新面板
const refreshPanel = async (panel: BrowserPanel) => {
  const webview = webviews.value.get(panel.webviewLabel);
  if (webview) {
    // 先移除所有事件监听器
    const webviewWithCleanup = webview as Webview & { _unlisteners?: (() => void)[] };
    if (webviewWithCleanup._unlisteners) {
      webviewWithCleanup._unlisteners.forEach(unlisten => unlisten());
    }
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
    // 先移除所有事件监听器
    const webviewWithCleanup = webview as Webview & { _unlisteners?: (() => void)[] };
    if (webviewWithCleanup._unlisteners) {
      webviewWithCleanup._unlisteners.forEach(unlisten => unlisten());
    }
    await webview.close();
    webviews.value.delete(panel.webviewLabel);
  }
  panel.url = '';
  panel.isActive = false;
};

// 刷新其他面板（用于登录后同步状态）
const refreshOtherPanels = async (currentPanel: BrowserPanel) => {
  // 遍历所有面板，刷新除当前面板外的其他面板
  for (const panel of panels.value) {
    if (panel.id !== currentPanel.id && panel.isActive) {
      // 使用 initialUrl 重新加载，确保使用初始设置的地址
      panel.url = panel.initialUrl;
      const webview = webviews.value.get(panel.webviewLabel);
      if (webview) {
        // 先移除所有事件监听器
        const webviewWithCleanup = webview as Webview & { _unlisteners?: (() => void)[] };
        if (webviewWithCleanup._unlisteners) {
          webviewWithCleanup._unlisteners.forEach(unlisten => unlisten());
        }
        // 关闭并重新创建 webview 来实现刷新
        await webview.close();
      }
      const container = containerRefs.value.get(panel.id);
      if (container) {
        await initWebview(panel, container);
      }
    }
  }
};

// 刷新所有面板
const refreshAllPanels = async () => {
  for (const panel of panels.value) {
    if (panel.isActive) {
      // 使用 initialUrl 重新加载
      panel.url = panel.initialUrl;
      const webview = webviews.value.get(panel.webviewLabel);
      if (webview) {
        // 先移除所有事件监听器
        const webviewWithCleanup = webview as Webview & { _unlisteners?: (() => void)[] };
        if (webviewWithCleanup._unlisteners) {
          webviewWithCleanup._unlisteners.forEach(unlisten => unlisten());
        }
        await webview.close();
      }
      const container = containerRefs.value.get(panel.id);
      if (container) {
        await initWebview(panel, container);
      }
    }
  }
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
      // 先移除所有事件监听器
      const webviewWithCleanup = webview as Webview & { _unlisteners?: (() => void)[] };
      if (webviewWithCleanup._unlisteners) {
        webviewWithCleanup._unlisteners.forEach(unlisten => unlisten());
      }
      await webview.close();
    } catch (error) {
      console.error(`关闭 Webview ${label} 失败:`, error);
    }
  }
  webviews.value.clear();
};

// 更新 Webview 位置和大小
const updateWebviewPositions = async () => {
  // 使用 requestAnimationFrame 优化性能
  requestAnimationFrame(async () => {
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
  });
};

// 存储全局事件监听器
let externalLinkUnlisten: (() => void) | null = null;

onMounted(async () => {
  initAllWebviews();

  // 监听窗口大小变化
  window.addEventListener('resize', updateWebviewPositions);
  // 监听拖拽事件
  document.addEventListener('mousemove', handleMouseMove);
  document.addEventListener('mouseup', stopDrag);

  // 监听来自 webview 的外部链接打开事件
  externalLinkUnlisten = await listen<{ url: string }>('open-external-link', (event) => {
    const url = event.payload.url;
    console.log('收到外部链接打开请求:', url);
    // 调用 Rust 命令打开外部链接
    invoke('open_external_url', { url }).catch((err) => {
      console.error('打开外部链接失败:', err);
    });
  });
});

onUnmounted(() => {
  cleanupAllWebviews();
  window.removeEventListener('resize', updateWebviewPositions);
  document.removeEventListener('mousemove', handleMouseMove);
  document.removeEventListener('mouseup', stopDrag);

  // 移除全局事件监听器
  if (externalLinkUnlisten) {
    externalLinkUnlisten();
  }
});
</script>

<template>
  <div class="split-screen-browser">
    <div class="browser-header">
      <div class="header-left">
        <h2 class="browser-title">🖥️ 四宫格浏览器</h2>
        <p class="browser-subtitle">基于 Tauri Webview，拖拽分隔条调整大小</p>
      </div>
      <div class="header-right">
        <label class="auto-sync-toggle">
          <input
            type="checkbox"
            v-model="autoSyncEnabled"
          />
          <span class="toggle-text">自动同步</span>
        </label>
        <button
          @click="refreshAllPanels"
          class="btn-refresh-all"
          title="使用初始地址刷新所有窗口"
        >
          🔄 刷新全部
        </button>
      </div>
    </div>

    <div
      ref="gridContainerRef"
      class="grid-container"
      :class="{ 'is-dragging': isDraggingHorizontal || isDraggingVertical }"
      :style="{
        gridTemplateColumns: `${horizontalSplit}fr ${100 - horizontalSplit}fr`,
        gridTemplateRows: `${verticalSplit}fr ${100 - verticalSplit}fr`
      }"
    >
      <!-- 左上格子 -->
      <div
        class="browser-panel panel-1"
        :ref="(el) => { if (el) containerRefs.set(1, el as HTMLElement) }"
      >
        <div class="address-bar">
          <div class="panel-number">#1</div>
          <input
            type="text"
            v-model="panels[0].url"
            @keyup.enter="handleUrlSubmit(panels[0])"
            @focus="editingPanels.add(1)"
            @blur="editingPanels.delete(1)"
            placeholder="输入网址，按回车访问..."
            class="url-input"
          />
          <button
            @click="handleUrlSubmit(panels[0])"
            class="btn-go"
            :disabled="!panels[0].url.trim()"
          >
            访问
          </button>
          <button
            v-if="panels[0].isActive"
            @click="refreshPanel(panels[0])"
            class="btn-icon"
            title="刷新"
          >
            🔄
          </button>
          <button
            v-if="panels[0].isActive"
            @click="clearPanel(panels[0])"
            class="btn-icon"
            title="清空"
          >
            🗑️
          </button>
          <button
            v-if="panels[0].isActive"
            @click="refreshOtherPanels(panels[0])"
            class="btn-sync"
            title="登录后使用初始地址刷新其他窗口"
          >
            🔗 同步其他
          </button>
        </div>
        <div class="webview-container">
          <div v-if="!panels[0].isActive" class="empty-state">
            <div class="empty-icon">🌐</div>
            <p>输入网址开始浏览</p>
          </div>
        </div>
      </div>

      <!-- 右上格子 -->
      <div
        class="browser-panel panel-2"
        :ref="(el) => { if (el) containerRefs.set(2, el as HTMLElement) }"
      >
        <div class="address-bar">
          <div class="panel-number">#2</div>
          <input
            type="text"
            v-model="panels[1].url"
            @keyup.enter="handleUrlSubmit(panels[1])"
            @focus="editingPanels.add(2)"
            @blur="editingPanels.delete(2)"
            placeholder="输入网址，按回车访问..."
            class="url-input"
          />
          <button
            @click="handleUrlSubmit(panels[1])"
            class="btn-go"
            :disabled="!panels[1].url.trim()"
          >
            访问
          </button>
          <button
            v-if="panels[1].isActive"
            @click="refreshPanel(panels[1])"
            class="btn-icon"
            title="刷新"
          >
            🔄
          </button>
          <button
            v-if="panels[1].isActive"
            @click="clearPanel(panels[1])"
            class="btn-icon"
            title="清空"
          >
            🗑️
          </button>
          <button
            v-if="panels[1].isActive"
            @click="refreshOtherPanels(panels[1])"
            class="btn-sync"
            title="登录后使用初始地址刷新其他窗口"
          >
            🔗 同步其他
          </button>
        </div>
        <div class="webview-container">
          <div v-if="!panels[1].isActive" class="empty-state">
            <div class="empty-icon">🌐</div>
            <p>输入网址开始浏览</p>
          </div>
        </div>
      </div>

      <!-- 左下格子 -->
      <div
        class="browser-panel panel-3"
        :ref="(el) => { if (el) containerRefs.set(3, el as HTMLElement) }"
      >
        <div class="address-bar">
          <div class="panel-number">#3</div>
          <input
            type="text"
            v-model="panels[2].url"
            @keyup.enter="handleUrlSubmit(panels[2])"
            @focus="editingPanels.add(3)"
            @blur="editingPanels.delete(3)"
            placeholder="输入网址，按回车访问..."
            class="url-input"
          />
          <button
            @click="handleUrlSubmit(panels[2])"
            class="btn-go"
            :disabled="!panels[2].url.trim()"
          >
            访问
          </button>
          <button
            v-if="panels[2].isActive"
            @click="refreshPanel(panels[2])"
            class="btn-icon"
            title="刷新"
          >
            🔄
          </button>
          <button
            v-if="panels[2].isActive"
            @click="clearPanel(panels[2])"
            class="btn-icon"
            title="清空"
          >
            🗑️
          </button>
          <button
            v-if="panels[2].isActive"
            @click="refreshOtherPanels(panels[2])"
            class="btn-sync"
            title="登录后使用初始地址刷新其他窗口"
          >
            🔗 同步其他
          </button>
        </div>
        <div class="webview-container">
          <div v-if="!panels[2].isActive" class="empty-state">
            <div class="empty-icon">🌐</div>
            <p>输入网址开始浏览</p>
          </div>
        </div>
      </div>

      <!-- 右下格子 -->
      <div
        class="browser-panel panel-4"
        :ref="(el) => { if (el) containerRefs.set(4, el as HTMLElement) }"
      >
        <div class="address-bar">
          <div class="panel-number">#4</div>
          <input
            type="text"
            v-model="panels[3].url"
            @keyup.enter="handleUrlSubmit(panels[3])"
            @focus="editingPanels.add(4)"
            @blur="editingPanels.delete(4)"
            placeholder="输入网址，按回车访问..."
            class="url-input"
          />
          <button
            @click="handleUrlSubmit(panels[3])"
            class="btn-go"
            :disabled="!panels[3].url.trim()"
          >
            访问
          </button>
          <button
            v-if="panels[3].isActive"
            @click="refreshPanel(panels[3])"
            class="btn-icon"
            title="刷新"
          >
            🔄
          </button>
          <button
            v-if="panels[3].isActive"
            @click="clearPanel(panels[3])"
            class="btn-icon"
            title="清空"
          >
            🗑️
          </button>
          <button
            v-if="panels[3].isActive"
            @click="refreshOtherPanels(panels[3])"
            class="btn-sync"
            title="登录后使用初始地址刷新其他窗口"
          >
            🔗 同步其他
          </button>
        </div>
        <div class="webview-container">
          <div v-if="!panels[3].isActive" class="empty-state">
            <div class="empty-icon">🌐</div>
            <p>输入网址开始浏览</p>
          </div>
        </div>
      </div>

      <!-- 水平分隔条（左右分割） -->
      <div
        class="splitter splitter-horizontal"
        :style="{ left: horizontalSplit + '%' }"
        @mousedown="startDragHorizontal"
      >
        <div class="splitter-handle"></div>
      </div>

      <!-- 垂直分隔条（上下分割） -->
      <div
        class="splitter splitter-vertical"
        :style="{ top: verticalSplit + '%' }"
        @mousedown="startDragVertical"
      >
        <div class="splitter-handle"></div>
      </div>

      <!-- 交叉点 -->
      <div
        class="splitter-cross"
        :style="{ left: horizontalSplit + '%', top: verticalSplit + '%' }"
        @mousedown="startDragCross"
      ></div>
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
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.header-left {
  display: flex;
  flex-direction: column;
}

.btn-refresh-all {
  padding: 10px 20px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: #fff;
  border: none;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
  box-shadow: 0 4px 12px rgba(102, 126, 234, 0.3);
}

.btn-refresh-all:hover {
  transform: translateY(-2px);
  box-shadow: 0 6px 16px rgba(102, 126, 234, 0.4);
}

.header-right {
  display: flex;
  align-items: center;
  gap: 16px;
}

.auto-sync-toggle {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  padding: 8px 12px;
  background: rgba(102, 126, 234, 0.1);
  border-radius: 8px;
  transition: all 0.2s;
}

.auto-sync-toggle:hover {
  background: rgba(102, 126, 234, 0.2);
}

.auto-sync-toggle input[type="checkbox"] {
  width: 18px;
  height: 18px;
  cursor: pointer;
  accent-color: #667eea;
}

.toggle-text {
  font-size: 14px;
  color: #667eea;
  font-weight: 500;
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
  position: relative;
  padding: 12px;
  overflow: hidden;
  gap: 0;
}

.grid-container.is-dragging {
  cursor: col-resize;
}

.grid-container.is-dragging .splitter-vertical {
  cursor: row-resize;
}

.browser-panel {
  background: #fff;
  overflow: hidden;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.1);
  display: flex;
  flex-direction: column;
  transition: all 0.3s ease;
  border: 2px solid #667eea;
  position: relative;
}

.panel-1 {
  border-top-left-radius: 12px;
  grid-column: 1;
  grid-row: 1;
}

.panel-2 {
  border-top-right-radius: 12px;
  grid-column: 2;
  grid-row: 1;
}

.panel-3 {
  border-bottom-left-radius: 12px;
  grid-column: 1;
  grid-row: 2;
}

.panel-4 {
  border-bottom-right-radius: 12px;
  grid-column: 2;
  grid-row: 2;
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

.btn-sync {
  padding: 6px 12px;
  background: linear-gradient(135deg, #48bb78 0%, #38a169 100%);
  color: #fff;
  border: none;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
  font-size: 13px;
  font-weight: 500;
  white-space: nowrap;
}

.btn-sync:hover {
  transform: translateY(-1px);
  box-shadow: 0 4px 12px rgba(72, 187, 120, 0.3);
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

/* 分隔条样式 */
.splitter {
  position: absolute;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
}

.splitter-horizontal {
  top: 12px;
  bottom: 12px;
  width: 16px;
  margin-left: -8px;
  cursor: col-resize;
  background: linear-gradient(
    to right,
    transparent 30%,
    rgba(102, 126, 234, 0.8) 30%,
    rgba(102, 126, 234, 0.8) 70%,
    transparent 70%
  );
}

.splitter-vertical {
  left: 12px;
  right: 12px;
  height: 16px;
  margin-top: -8px;
  cursor: row-resize;
  background: linear-gradient(
    to bottom,
    transparent 30%,
    rgba(102, 126, 234, 0.8) 30%,
    rgba(102, 126, 234, 0.8) 70%,
    transparent 70%
  );
}

.splitter-handle {
  background: #667eea;
  border-radius: 4px;
  box-shadow: 0 2px 8px rgba(102, 126, 234, 0.4);
  transition: all 0.2s;
}

.splitter-horizontal .splitter-handle {
  width: 6px;
  height: 50px;
  border-radius: 3px;
}

.splitter-vertical .splitter-handle {
  width: 50px;
  height: 6px;
  border-radius: 3px;
}

.splitter:hover .splitter-handle {
  background: #764ba2;
  box-shadow: 0 4px 12px rgba(118, 75, 162, 0.5);
}

.splitter-horizontal:hover .splitter-handle {
  width: 10px;
}

.splitter-vertical:hover .splitter-handle {
  height: 10px;
}

/* 交叉点 */
.splitter-cross {
  position: absolute;
  width: 24px;
  height: 24px;
  margin-left: -12px;
  margin-top: -12px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  border-radius: 50%;
  z-index: 101;
  cursor: move;
  transition: all 0.2s;
  box-shadow: 0 4px 12px rgba(102, 126, 234, 0.5);
  border: 3px solid #fff;
}

.splitter-cross:hover {
  transform: scale(1.3);
  box-shadow: 0 6px 20px rgba(102, 126, 234, 0.7);
}

.splitter-cross::before,
.splitter-cross::after {
  content: '';
  position: absolute;
  background: #fff;
}

.splitter-cross::before {
  width: 12px;
  height: 3px;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
}

.splitter-cross::after {
  width: 3px;
  height: 12px;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
}
</style>
