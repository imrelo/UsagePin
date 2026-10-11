import "./shared/Window.css";
import UsagePin from "./UsagePin";
import Settings from "./settings/Settings";

const isSettingsWindow = new URLSearchParams(window.location.search).get("window") === "settings";

function App() {
  return isSettingsWindow ? <Settings /> : <UsagePin />;
}

export default App;
