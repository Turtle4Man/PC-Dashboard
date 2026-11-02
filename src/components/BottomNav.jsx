import { ButtonsIcon, SettingsIcon, ConfigIcon } from "./icons";

const TABS = [
  { id: "home", label: "Accueil", Icon: ButtonsIcon },
  { id: "settings", label: "Réglages", Icon: SettingsIcon },
  { id: "hardware", label: "Configuration matérielle", Icon: ConfigIcon },
];

export default function BottomNav({ tab, onSelect }) {
  return (
    <nav className="bottomnav" aria-label="Navigation">
      {TABS.map(({ id, label, Icon }) => (
        <button
          key={id}
          type="button"
          title={label}
          aria-label={label}
          className={`navbtn ${tab === id ? "navbtn--active" : ""}`}
          onClick={() => onSelect(id)}
        >
          <Icon size={25} />
        </button>
      ))}
    </nav>
  );
}
