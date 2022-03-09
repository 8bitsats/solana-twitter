import "solana-wallets-vue/styles.css";
import "./main.css";
import { createApp } from "vue";
import App from "./App.vue";

import { createRouter, createWebHashHistory } from "vue-router";
import routes from "./routes";
const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

createApp(App).use(router).mount("#app");
