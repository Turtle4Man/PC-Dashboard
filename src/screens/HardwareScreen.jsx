import { useHardware } from "../store/useHardware";

export default function HardwareScreen() {
  const { hardware, error } = useHardware();

  return (
    <div className="hardware">
      <h1>Configuration matérielle</h1>

      {error && !hardware && <p className="hardware__error">{error}</p>}
      {!hardware && !error && <p className="hardware__muted">Chargement…</p>}

      {hardware && (
        <>
          <section>
            <h2>Composants</h2>
            <div className="components">
              {hardware.components.map((component, index) => (
                <article
                  className="component"
                  key={`${component.label}-${index}`}
                >
                  <span className="component__label">{component.label}</span>
                  <span className="component__name">{component.name}</span>
                  {component.detail && (
                    <span className="component__detail">
                      {component.detail}
                    </span>
                  )}
                </article>
              ))}
            </div>
          </section>

          <section>
            <h2>Périphériques connectés</h2>
            {hardware.peripherals.length === 0 && (
              <p className="hardware__muted">Aucun périphérique détecté.</p>
            )}
            <div className="peripheral-groups">
              {hardware.peripherals.map((group) => (
                <div className="peripheral-group" key={group.label}>
                  <h3 className="peripheral-group__label">{group.label}</h3>
                  <ul className="peripheral-group__items">
                    {group.items.map((item) => (
                      <li className="peripheral-group__item" key={item}>
                        {item}
                      </li>
                    ))}
                  </ul>
                </div>
              ))}
            </div>
          </section>
        </>
      )}
    </div>
  );
}
