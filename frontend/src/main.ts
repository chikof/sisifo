import "./app.css";
import App from "./App.svelte";
import { mount } from "svelte";

(function() {
	try {
		const stored = localStorage.getItem("sisi-theme");
		if (stored === "dark" || stored === "light") {
			document.documentElement.setAttribute("data-theme", stored);
		} else if (window.matchMedia("(prefers-color-scheme: dark)").matches) {
			document.documentElement.setAttribute("data-theme", "dark");
		} else {
			document.documentElement.setAttribute("data-theme", "light");
		}
	} catch { }
})();

const app = mount(App, { target: document.body });

export default app;
