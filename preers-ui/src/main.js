import { createApp } from 'vue';
import App from './App.vue';
import './assets/styles/app.css';
import axios from 'axios';

const baseUrl = 'http://localhost:9843';
// 可以在此处配置 axios 的 baseURL
axios.defaults.baseURL = baseUrl;

//axios.defaults.withCredentials = true;

document.title = 'Preers';

const app = createApp(App);

app.config.globalProperties.$axios = axios; // 将 axios 添加到全局属性，以便在组件中使用 this.$axios

app.mount('#app');