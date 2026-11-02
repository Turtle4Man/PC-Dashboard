import { useVitals } from "../store/useVitals";
import { formatMs, latencyLevel, percentLevel, pingLevel } from "../lib/format";

function StatRow({ label, value, level, pending }) {
  return (
    <div className={`stat-row level-${level}`}>
      <span className="stat-row__label">{label}</span>
      <span className="stat-row__val">{pending ? "…" : value}</span>
    </div>
  );
}

export default function StatsList() {
  const { vitals } = useVitals();
  const pending = !vitals;

  const cpu = vitals?.cpu?.global ?? 0;
  const gpus = vitals?.gpus ?? [];
  const gpu = gpus.length > 0 ? Math.max(...gpus.map((g) => g.usage)) : 0;
  const mem = vitals?.mem;
  const ram = mem?.total ? (mem.used / mem.total) * 100 : 0;
  const nets = vitals?.net_ping ?? [];
  const allDisks = vitals?.disks ?? [];
  const disks = allDisks
    .filter((d) => d.name !== "_Total")
    .sort((a, b) => a.name.localeCompare(b.name, "fr", { sensitivity: "base" }));
  const list = disks.length > 0 ? disks : allDisks;

  return (
    <div className="stats">
      <div className="stat-group stat-group--system">
        <StatRow
          label="CPU"
          pending={pending}
          level={percentLevel(cpu)}
          value={`${Math.round(cpu)} %`}
        />
        <StatRow
          label="GPU"
          pending={pending}
          level={percentLevel(gpu)}
          value={`${Math.round(gpu)} %`}
        />
        <StatRow
          label="RAM"
          pending={pending}
          level={percentLevel(ram)}
          value={`${Math.round(ram)} %`}
        />
      </div>
      <div className="stat-group stat-group--net">
        {nets.length <= 1 ? (
          <StatRow
            label="Réseau"
            pending={pending || nets.length === 0}
            level={nets.length === 0 ? "bad" : pingLevel(nets[0].ms)}
            value={nets.length === 0 ? "—" : formatMs(nets[0].ms)}
          />
        ) : (
          <>
            <div className="stat-row">
              <span className="stat-row__label">Réseau</span>
            </div>
            {nets.map((net) => (
              <div className={`stat-row stat-row--sub level-${pingLevel(net.ms)}`} key={net.name}>
                <span className="stat-row__label">{net.name}</span>
                <span className="stat-row__val">{formatMs(net.ms)}</span>
              </div>
            ))}
          </>
        )}
      </div>
      <div className="stat-group stat-group--disk">
        {list.length <= 1 ? (
          <StatRow
            label="Disque"
            pending={pending || list.length === 0}
            level={list.length === 0 ? "bad" : latencyLevel(list[0].resp_ms)}
            value={list.length === 0 ? "—" : formatMs(list[0].resp_ms)}
          />
        ) : (
          <>
            <div className="stat-row">
              <span className="stat-row__label">Disques</span>
            </div>
            {list.map((disk) => (
              <div
                className={`stat-row stat-row--sub level-${latencyLevel(disk.resp_ms)}`}
                key={disk.name}
              >
                <span className="stat-row__label">{disk.name}</span>
                <span className="stat-row__val">{formatMs(disk.resp_ms)}</span>
              </div>
            ))}
          </>
        )}
      </div>
    </div>
  );
}
