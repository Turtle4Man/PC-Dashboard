import { useEffect, useState } from "react";
import { useVitals } from "../store/useVitals";
import { mediaNext, mediaPrevious, openMediaSource, toggleMedia } from "../lib/ipc";
import { NextIcon, PauseIcon, PlayIcon, PreviousIcon } from "./icons";

export default function MediaPill() {
  const { vitals } = useVitals();
  const media = vitals?.media;
  const playing = media?.playing;
  const hasSession = playing === true || playing === false;
  const [pending, setPending] = useState(null);
  const [art, setArt] = useState(null);

  // The backend only sends the thumbnail when it changes (absent = unchanged,
  // null = cleared), so the art is cached here instead of being re-sent.
  const thumbUpdate = media && "thumb" in media ? media.thumb : undefined;
  useEffect(() => {
    if (!hasSession) {
      setArt(null);
    } else if (thumbUpdate !== undefined) {
      setArt(thumbUpdate);
    }
  }, [hasSession, thumbUpdate]);

  async function run(kind, action) {
    setPending(kind);
    try {
      await action();
    } catch (error) {
      console.error(error);
    } finally {
      setPending(null);
    }
  }

  return (
    <div className={`mediapill${hasSession ? "" : " mediapill--idle"}`}>
      {art && (
        <div
          className="mediapill__bg"
          style={{ backgroundImage: `url("${art}")` }}
          aria-hidden="true"
        />
      )}
      <div className="mediapill__meta">
        <button
          type="button"
          className="mediapill__title"
          title={hasSession ? "Ouvrir la source" : undefined}
          onClick={() => run("source", openMediaSource)}
          disabled={!hasSession || pending !== null}
        >
          {media?.title || (vitals ? "Aucune lecture" : "…")}
        </button>
        {media?.artist && (
          <span className="mediapill__artist">{media.artist}</span>
        )}
      </div>
      <div className="mediapill__controls">
        <button
          type="button"
          title="Piste précédente"
          onClick={() => run("prev", mediaPrevious)}
          disabled={!hasSession || pending !== null}
        >
          <PreviousIcon size={30} />
        </button>
        <button
          type="button"
          title={playing === true ? "Mettre en pause" : "Reprendre la lecture"}
          onClick={() => run("toggle", toggleMedia)}
          disabled={!hasSession || pending !== null}
        >
          {playing === false ? <PlayIcon size={30} /> : <PauseIcon size={30} />}
        </button>
        <button
          type="button"
          title="Piste suivante"
          onClick={() => run("next", mediaNext)}
          disabled={!hasSession || pending !== null}
        >
          <NextIcon size={30} />
        </button>
      </div>
    </div>
  );
}
