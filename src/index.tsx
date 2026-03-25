import { render } from "solid-js/web";
import { HashRouter, Route } from "@solidjs/router";
import App from "./App";
import { SettingsWindow } from "./components/settings/SettingsWindow";

render(
  () => (
    <HashRouter>
      <Route path="/" component={App} />
      <Route path="/settings" component={SettingsWindow} />
    </HashRouter>
  ),
  document.getElementById("root") as HTMLElement,
);
