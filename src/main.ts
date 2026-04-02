import { createApp } from "vue";
import App from "./App.vue";
import "./styles.css";
import AntDesignVue from 'ant-design-vue';

// 创建 Vue 应用实例
const app = createApp(App);

// 全局注册 Ant Design Vue 组件
app.use(AntDesignVue);

// 挂载应用
app.mount("#app");
