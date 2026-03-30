<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

// 定义响应式数据
const greetMsg = ref("");
const name = ref("");

// 调用 Tauri 命令
async function greet() {
  try {
    greetMsg.value = await invoke("greet", { name: name.value });
  } catch (error) {
    console.error("调用 Tauri 命令失败:", error);
    greetMsg.value = "错误: 无法连接到 Tauri 后端，请确保在 Tauri 应用窗口中运行";
  }
}
</script>

<template>
  <main class="container">
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

    <form class="row" @submit.prevent="greet">
      <input
        id="greet-input"
        v-model="name"
        placeholder="请输入你的名字..."
      />
      <button type="submit">打招呼</button>
      <button type="reset">重置</button>
    </form>

    <p v-if="greetMsg">{{ greetMsg }}</p>
  </main>
</template>

<style scoped>
.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: filter 300ms;
}

.logo:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #42b883);
}
</style>
