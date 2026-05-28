import { SettingsWindow } from "./components/SettingsWindow";
import { WidgetWindow } from "./components/WidgetWindow";

export function App() {
  const params = new URLSearchParams(window.location.search);
  const view = params.get("window");

  return view === "settings" ? <SettingsWindow /> : <WidgetWindow />;
}
