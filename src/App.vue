<script setup lang="ts">
import { ref } from "vue";
import FilePreview from "./components/FilePreview.vue";

// 当前页面
const currentPage = ref<'home' | 'filePreview'>('home');

// 切换页面
const switchPage = (page: 'home' | 'filePreview') => {
  currentPage.value = page;
};
</script>

<template>
  <div class="app-container">
    <!-- 导航栏 -->
    <nav class="navbar">
      <div class="nav-brand">🚀 Tauri App</div>
      <div class="nav-links">
        <button 
          class="nav-link" 
          :class="{ active: currentPage === 'home' }"
          @click="switchPage('home')"
        >
          首页
        </button>
        <button 
          class="nav-link" 
          :class="{ active: currentPage === 'filePreview' }"
          @click="switchPage('filePreview')"
        >
          📦 文件预览
        </button>
      </div>
    </nav>

    <!-- 页面内容 -->
    <main class="main-content">
      <!-- 首页 -->
      <div v-if="currentPage === 'home'" class="home-page">
        <h1>欢迎使用 Tauri + Vue3 + TypeScript! 🚀</h1>
        
        <div class="row">
          <a href="https://tauri.app" target="_blank">
            <img src="/tauri.svg" class="logo tauri" alt="Tauri logo" />
          </a>
          <a href="https://vuejs.org/" target="_blank">
            <img src="/vue.svg" class="logo vue" alt="Vue logo" />
          </a>
        </div>

        <p>点击 Tauri 和 Vue 图标了解更多</p>
        
        <div class="feature-cards">
          <div class="card" @click="switchPage('filePreview')">
            <div class="card-icon">📦</div>
            <h3>文件预览</h3>
            <p>支持拖拽上传压缩包，密码解压，在线预览文件内容</p>
          </div>
        </div>
      </div>

      <!-- 文件预览页面 -->
      <div v-else-if="currentPage === 'filePreview'">
        <FilePreview />
      </div>
    </main>
  </div>
</template>

<style scoped>
.app-container {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

/* 导航栏 */
.navbar {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0 24px;
  height: 64px;
  background: #fff;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  position: sticky;
  top: 0;
  z-index: 100;
}

.nav-brand {
  font-size: 20px;
  font-weight: 600;
  color: #1890ff;
}

.nav-links {
  display: flex;
  gap: 8px;
}

.nav-link {
  padding: 8px 16px;
  border: none;
  background: transparent;
  color: #666;
  font-size: 14px;
  cursor: pointer;
  border-radius: 4px;
  transition: all 0.3s;
}

.nav-link:hover {
  background: #f0f0f0;
  color: #1890ff;
}

.nav-link.active {
  background: #e6f7ff;
  color: #1890ff;
  font-weight: 500;
}

/* 主内容区 */
.main-content {
  flex: 1;
  padding: 24px;
}

/* 首页 */
.home-page {
  text-align: center;
  max-width: 800px;
  margin: 0 auto;
}

.home-page h1 {
  margin-bottom: 24px;
  color: #333;
}

.row {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 24px;
  margin-bottom: 24px;
}

.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: filter 0.3s;
}

.logo:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #42b883);
}

/* 功能卡片 */
.feature-cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
  gap: 24px;
  margin-top: 48px;
}

.card {
  background: #fff;
  border-radius: 8px;
  padding: 24px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
  cursor: pointer;
  transition: all 0.3s;
  text-align: left;
}

.card:hover {
  transform: translateY(-4px);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.15);
}

.card-icon {
  font-size: 48px;
  margin-bottom: 16px;
}

.card h3 {
  margin-bottom: 8px;
  color: #333;
}

.card p {
  color: #666;
  font-size: 14px;
  line-height: 1.5;
}
</style>
