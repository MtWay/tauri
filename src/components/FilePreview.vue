<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { unzip, strFromU8 } from 'fflate';
import { listen } from '@tauri-apps/api/event';

// 定义事件
const emit = defineEmits<{
  fileSelected: [files: File[]];
}>();

// 解压结果类型
interface ExtractedFile {
  name: string;
  content: Uint8Array;
  type: string;
  isText: boolean;
}

// 响应式数据
const isDragging = ref(false);
const selectedFile = ref<File | null>(null);

// 硬编码的解压密码（如需修改，请更改此处）
const ARCHIVE_PASSWORD = 'test';

const isLoading = ref(false);
const errorMsg = ref('');
const extractedFiles = ref<ExtractedFile[]>([]);
const previewFile = ref<{ name: string; content: string; type: string } | null>(null);
const fileInputRef = ref<HTMLInputElement | null>(null);
const isPreviewModalOpen = ref(false);

// 支持的压缩包格式
const supportedFormats = ['.zip', '.7z'];

// 检查文件是否为压缩包
const isArchiveFile = (filename: string): boolean => {
  const lowerName = filename.toLowerCase();
  return supportedFormats.some(format => lowerName.endsWith(format));
};

// 获取文件类型
const getFileType = (filename: string): string => {
  const ext = filename.split('.').pop()?.toLowerCase() || '';
  const typeMap: Record<string, string> = {
    'txt': 'text/plain',
    'md': 'text/markdown',
    'json': 'application/json',
    'js': 'text/javascript',
    'ts': 'text/typescript',
    'vue': 'text/html',
    'html': 'text/html',
    'css': 'text/css',
    'xml': 'text/xml',
    'png': 'image/png',
    'jpg': 'image/jpeg',
    'jpeg': 'image/jpeg',
    'gif': 'image/gif',
    'svg': 'image/svg+xml',
    'pdf': 'application/pdf'
  };
  return typeMap[ext] || 'application/octet-stream';
};

// 判断是否为文本文件
const isTextFile = (filename: string, content: Uint8Array): boolean => {
  // 根据扩展名判断
  const textExtensions = [
    '.txt', '.md', '.json', '.js', '.ts', '.vue', '.html', '.css',
    '.xml', '.yaml', '.yml', '.toml', '.ini', '.conf', '.config',
    '.rs', '.py', '.java', '.c', '.cpp', '.h', '.hpp', '.go',
    '.rb', '.php', '.swift', '.kt', '.scala', '.r', '.m',
    '.sh', '.bash', '.zsh', '.ps1', '.bat', '.cmd',
    '.log', '.csv', '.tsv'
  ];
  
  const lowerName = filename.toLowerCase();
  if (textExtensions.some(ext => lowerName.endsWith(ext))) {
    return true;
  }

  // 检查内容是否为文本（简单的启发式检测）
  if (content.length === 0) {
    return true;
  }

  // 检查前 1024 字节是否包含空字节（二进制文件通常包含空字节）
  const checkLen = Math.min(content.length, 1024);
  for (let i = 0; i < checkLen; i++) {
    if (content[i] === 0) {
      return false;
    }
  }

  // 检查是否为有效的 UTF-8
  try {
    new TextDecoder('utf-8', { fatal: true }).decode(content.slice(0, checkLen));
    return true;
  } catch {
    return false;
  }
};

// 拖拽计数器，用于处理 dragenter/dragleave 的闪烁问题
let dragCounter = 0;

// 处理拖拽进入
const handleDragEnter = (e: DragEvent) => {
  e.preventDefault();
  dragCounter++;
  if (dragCounter === 1) {
    isDragging.value = true;
  }
};

// 处理拖拽离开
const handleDragLeave = (e: DragEvent) => {
  e.preventDefault();
  dragCounter--;
  if (dragCounter === 0) {
    isDragging.value = false;
  }
};

// 处理拖拽悬停
const handleDragOver = (e: DragEvent) => {
  e.preventDefault();
  // 必须设置 dropEffect 才能正常触发 drop 事件
  if (e.dataTransfer) {
    e.dataTransfer.dropEffect = 'copy';
  }
};

// 处理文件拖放
const handleDrop = (e: DragEvent) => {
  e.preventDefault();
  dragCounter = 0;
  isDragging.value = false;
  
  const files = Array.from(e.dataTransfer?.files || []);
  if (files.length > 0) {
    handleFileSelect(files[0]);
  }
};

// 处理文件选择
const handleFileInput = (e: Event) => {
  const target = e.target as HTMLInputElement;
  const files = target.files;
  if (files && files.length > 0) {
    handleFileSelect(files[0]);
  }
};

// 选择文件
const handleFileSelect = (file: File) => {
  if (!isArchiveFile(file.name)) {
    errorMsg.value = '请选择压缩包文件（支持 .zip, .7z 格式）';
    return;
  }
  
  selectedFile.value = file;
  errorMsg.value = '';
  extractedFiles.value = [];
  previewFile.value = null;
  emit('fileSelected', [file]);
};

// 触发文件选择
const triggerFileSelect = () => {
  fileInputRef.value?.click();
};

// 解压 ZIP 文件
const extractZip = async (uint8Array: Uint8Array): Promise<ExtractedFile[]> => {
  return new Promise((resolve, reject) => {
    unzip(uint8Array, (err, data) => {
      if (err) {
        reject(err);
        return;
      }
      
      const files: ExtractedFile[] = [];
      
      for (const [filename, content] of Object.entries(data)) {
        // 跳过文件夹
        if (filename.endsWith('/')) continue;
        
        const type = getFileType(filename);
        const isText = isTextFile(filename, content);
        
        files.push({
          name: filename,
          content,
          type,
          isText
        });
      }
      
      resolve(files);
    });
  });
};

// 解压 7z 文件
const extract7z = async (uint8Array: Uint8Array, password?: string): Promise<ExtractedFile[]> => {
  // 动态导入 7z-wasm
  const SevenZipModule = await import('7z-wasm');
  const sz = await SevenZipModule.default();
  
  // 写入压缩包数据到虚拟文件系统
  const archivePath = '/archive.7z';
  sz.FS.writeFile(archivePath, uint8Array);
  
  // 创建输出目录
  const outputDir = '/output';
  sz.FS.mkdir(outputDir);
  
  try {
    // 构建解压命令参数
    const args = ['x', archivePath, `-o${outputDir}`, '-y'];
    if (password) {
      args.push(`-p${password}`);
    } else {
      args.push('-p'); // 尝试空密码
    }
    
    // 执行解压
    sz.callMain(args);
    
    // 读取解压后的文件
    const files: ExtractedFile[] = [];
    
    const readDir = (dir: string, basePath: string = '') => {
      const entries = sz.FS.readdir(dir);
      
      for (const entry of entries) {
        if (entry === '.' || entry === '..') continue;
        
        const fullPath = `${dir}/${entry}`;
        const relativePath = basePath ? `${basePath}/${entry}` : entry;
        const stat = sz.FS.stat(fullPath);
        
        if (sz.FS.isDir(stat.mode)) {
          readDir(fullPath, relativePath);
        } else {
          const content = sz.FS.readFile(fullPath);
          const type = getFileType(relativePath);
          const isText = isTextFile(relativePath, content);
          
          files.push({
            name: relativePath,
            content,
            type,
            isText
          });
        }
      }
    };
    
    readDir(outputDir);
    
    return files;
  } finally {
    // 清理虚拟文件系统
    try {
      sz.FS.unlink(archivePath);
    } catch (e) {
      // 忽略错误
    }
  }
};

// 解压文件
const extractArchive = async () => {
  if (!selectedFile.value) return;
  
  isLoading.value = true;
  errorMsg.value = '';
  extractedFiles.value = [];
  
  try {
    const file = selectedFile.value;
    const arrayBuffer = await file.arrayBuffer();
    const uint8Array = new Uint8Array(arrayBuffer);
    const fileName = file.name.toLowerCase();
    
    let files: ExtractedFile[] = [];
    
    if (fileName.endsWith('.zip')) {
      files = await extractZip(uint8Array);
    } else if (fileName.endsWith('.7z')) {
      // 使用硬编码密码解压 7z
      files = await extract7z(uint8Array, ARCHIVE_PASSWORD);
    } else {
      throw new Error('不支持的文件格式');
    }
    
    extractedFiles.value = files;
    
    if (files.length === 0) {
      errorMsg.value = '压缩包为空';
    }
  } catch (error) {
    console.error('解压失败:', error);
    if (error instanceof Error) {
      if (error.message.includes('password') || error.message.includes('encrypted')) {
        errorMsg.value = '解压失败：该压缩包需要密码或密码错误';
      } else if (error.message.includes('invalid') || error.message.includes('format')) {
        errorMsg.value = '解压失败：文件不是有效的压缩格式，或文件已损坏';
      } else {
        errorMsg.value = `解压失败：${error.message}`;
      }
    } else {
      errorMsg.value = '解压失败：未知错误';
    }
  } finally {
    isLoading.value = false;
  }
};

// 预览文件
const previewSelectedFile = (file: ExtractedFile) => {
  if (file.isText) {
    try {
      const text = strFromU8(file.content);
      previewFile.value = {
        name: file.name,
        content: text,
        type: file.type
      };
    } catch {
      // 如果解码失败，显示为二进制
      const hex = Array.from(file.content.slice(0, 100))
        .map(b => b.toString(16).padStart(2, '0'))
        .join(' ');
      previewFile.value = {
        name: file.name,
        content: `二进制文件（前100字节）：\n${hex}`,
        type: 'text/plain'
      };
    }
  } else {
    // 二进制文件显示为十六进制预览
    const hex = Array.from(file.content.slice(0, 100))
      .map(b => b.toString(16).padStart(2, '0'))
      .join(' ');
    previewFile.value = {
      name: file.name,
      content: `二进制文件（前100字节）：\n${hex}`,
      type: 'text/plain'
    };
  }
};

// 关闭预览
const closePreview = () => {
  isPreviewModalOpen.value = false;
  previewFile.value = null;
};

// 打开预览弹窗
const openPreviewModal = (file: ExtractedFile) => {
  previewSelectedFile(file);
  isPreviewModalOpen.value = true;
};

// 清空选择
const clearSelection = () => {
  selectedFile.value = null;
  errorMsg.value = '';
  extractedFiles.value = [];
  previewFile.value = null;
  isPreviewModalOpen.value = false;
  if (fileInputRef.value) {
    fileInputRef.value.value = '';
  }
};

// 格式化文件大小
const formatFileSize = (bytes: number): string => {
  if (bytes === 0) return '0 Bytes';
  const k = 1024;
  const sizes = ['Bytes', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
};

// Tauri 拖拽事件处理（用于桌面端）
let unlistenDrop: (() => void) | null = null;
let unlistenHover: (() => void) | null = null;
let unlistenCancel: (() => void) | null = null;

onMounted(async () => {
  // 使用 Tauri 事件监听文件拖放
  unlistenDrop = await listen<{ paths: string[] }>('tauri://file-drop', (event) => {
    console.log('文件拖放:', event.payload.paths);
    isDragging.value = false;
    const paths = event.payload.paths;
    if (paths.length > 0) {
      fetchFileFromPath(paths[0]);
    }
  });
  
  unlistenHover = await listen('tauri://file-drop-hover', () => {
    console.log('文件悬停');
    isDragging.value = true;
  });
  
  unlistenCancel = await listen('tauri://file-drop-cancelled', () => {
    console.log('拖放取消');
    isDragging.value = false;
  });
});

onUnmounted(() => {
  unlistenDrop?.();
  unlistenHover?.();
  unlistenCancel?.();
});

// 从路径获取文件
const fetchFileFromPath = async (path: string) => {
  try {
    console.log('读取文件:', path);
    // 使用 Tauri 的 fs API 读取文件
    const { readFile } = await import('@tauri-apps/plugin-fs');
    const content = await readFile(path);
    const fileName = path.split(/[\\/]/).pop() || 'unknown';
    
    console.log('文件读取成功:', fileName, '大小:', content.length);
    
    // 创建 File 对象
    const file = new File([content], fileName, { 
      type: 'application/octet-stream' 
    });
    
    handleFileSelect(file);
  } catch (error) {
    console.error('读取文件失败:', error);
    errorMsg.value = '读取文件失败，请尝试点击选择文件';
  }
};
</script>

<template>
  <div class="file-preview-container">
    <!-- 标题 -->
    <h2 class="title">📦 压缩包预览工具</h2>
    
    <!-- 拖拽区域 -->
    <div
      class="drop-zone"
      :class="{ 'dragging': isDragging, 'has-file': selectedFile }"
      @dragenter="handleDragEnter"
      @dragleave="handleDragLeave"
      @dragover="handleDragOver"
      @drop="handleDrop"
      @click="triggerFileSelect"
    >
      <input
        ref="fileInputRef"
        type="file"
        accept=".zip,.7z"
        style="display: none"
        @change="handleFileInput"
      />
      
      <div v-if="!selectedFile" class="drop-zone-content">
        <div class="icon">📁</div>
        <p class="hint">拖拽压缩包到此处，或点击选择文件</p>
        <p class="sub-hint">支持格式：.zip, .7z</p>
      </div>
      
      <div v-else class="file-info">
        <div class="icon">📦</div>
        <p class="filename">{{ selectedFile.name }}</p>
        <p class="filesize">{{ formatFileSize(selectedFile.size) }}</p>
        <a-button type="link" @click.stop="clearSelection">重新选择</a-button>
      </div>
    </div>
    
    <!-- 解压按钮 -->
    <div v-if="selectedFile" class="extract-section">
      <a-button
        type="primary"
        size="large"
        :loading="isLoading"
        @click="extractArchive"
      >
        {{ isLoading ? '解压中...' : '解压预览' }}
      </a-button>
    </div>
    
    <!-- 错误提示 -->
    <div v-if="errorMsg" class="error-message">
      <a-alert :message="errorMsg" type="error" show-icon />
    </div>
    
    <!-- 文件列表 -->
    <div v-if="extractedFiles.length > 0" class="file-list-section">
      <h3>📋 压缩包内容（共 {{ extractedFiles.length }} 个文件）</h3>
      <div class="file-list">
        <div
          v-for="file in extractedFiles"
          :key="file.name"
          class="file-item"
          @click="openPreviewModal(file)"
        >
          <span class="file-icon">📄</span>
          <span class="file-name">{{ file.name }}</span>
          <span class="file-type">{{ file.type }}</span>
          <a-button type="link" size="small">预览</a-button>
        </div>
      </div>
    </div>
    
    <!-- 文件预览弹窗 -->
    <a-modal
      v-model:open="isPreviewModalOpen"
      :title="previewFile?.name || '文件预览'"
      width="80%"
      :footer="null"
      @cancel="closePreview"
    >
      <div class="preview-content">
        <pre v-if="previewFile?.type.startsWith('text/') || previewFile?.type === 'application/json'">{{ previewFile?.content }}</pre>
        <div v-else class="binary-preview">{{ previewFile?.content }}</div>
      </div>
    </a-modal>
  </div>
</template>

<style scoped>
.file-preview-container {
  padding: 24px;
  max-width: 1200px;
  margin: 0 auto;
}

.title {
  text-align: center;
  margin-bottom: 24px;
  color: #1890ff;
}

.drop-zone {
  border: 2px dashed #d9d9d9;
  border-radius: 8px;
  padding: 48px;
  text-align: center;
  cursor: pointer;
  transition: all 0.3s;
  background: #fafafa;
}

.drop-zone:hover {
  border-color: #1890ff;
  background: #e6f7ff;
}

.drop-zone.dragging {
  border-color: #1890ff;
  background: #e6f7ff;
  transform: scale(1.02);
}

.drop-zone.has-file {
  border-color: #52c41a;
  background: #f6ffed;
}

.drop-zone-content .icon {
  font-size: 48px;
  margin-bottom: 16px;
}

.hint {
  font-size: 16px;
  color: #333;
  margin-bottom: 8px;
}

.sub-hint {
  font-size: 14px;
  color: #999;
}

.file-info .icon {
  font-size: 48px;
  margin-bottom: 16px;
}

.filename {
  font-size: 16px;
  font-weight: 500;
  color: #333;
  margin-bottom: 8px;
  word-break: break-all;
}

.filesize {
  font-size: 14px;
  color: #999;
  margin-bottom: 16px;
}

.extract-section {
  margin-top: 24px;
  text-align: center;
}

.error-message {
  margin-top: 16px;
}

.file-list-section {
  margin-top: 24px;
}

.file-list-section h3 {
  margin-bottom: 16px;
  color: #333;
}

.file-list {
  border: 1px solid #d9d9d9;
  border-radius: 8px;
  max-height: 400px;
  overflow-y: auto;
}

.file-item {
  display: flex;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid #f0f0f0;
  cursor: pointer;
  transition: background 0.3s;
}

.file-item:last-child {
  border-bottom: none;
}

.file-item:hover {
  background: #f5f5f5;
}

.file-icon {
  margin-right: 12px;
  font-size: 20px;
}

.file-name {
  flex: 1;
  font-size: 14px;
  color: #333;
  word-break: break-all;
}

.file-type {
  font-size: 12px;
  color: #999;
  margin-right: 12px;
}

.preview-content {
  max-height: 60vh;
  overflow: auto;
}

.preview-content pre {
  background: #f5f5f5;
  padding: 16px;
  border-radius: 4px;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 14px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-wrap: break-word;
}

.binary-preview {
  background: #f5f5f5;
  padding: 16px;
  border-radius: 4px;
  font-family: 'Consolas', 'Monaco', monospace;
  font-size: 14px;
  line-height: 1.5;
  white-space: pre-wrap;
  word-wrap: break-word;
  color: #666;
}
</style>
