import logo from "../../src-tauri/icons/icon.png";

export function Brand() {
  return (
    <div className="app-brand">
      <img src={logo} alt="" width="26" height="26" />
      GitPilot
    </div>
  );
}
