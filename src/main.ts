import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";
import { createAppI18n } from "./i18n";
import { createAppRouter } from "./router";
import "./styles/main.css";

createApp(App).use(createPinia()).use(createAppRouter()).use(createAppI18n()).mount("#app");
