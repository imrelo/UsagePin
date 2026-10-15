import "./MaterialIcon.css";

const icons = {
  settings: "\ue8b8",
  light_mode: "\ue518",
  close: "\ue5cd",
  opacity: "\ue91c",
  text_fields: "\ue262",
  mouse: "\ue323",
  check: "\ue5ca",
  expand_less: "\ue5ce",
  refresh: "\ue5d5",
  warning: "\ue002",
} as const;

export function MaterialIcon({ name }: { name: keyof typeof icons }) {
  return <span className="material-icon" aria-hidden="true">{icons[name]}</span>;
}
