import { useEffect, useState } from "react";
import { filtreDe, useReglages } from "./reglages";
import { cachedThumb, loadThumb } from "./thumbs";

/** A postage-stamp thumbnail, for the foot. */
export function MiniThumb({ src }: { src: string }) {
  const [url, setUrl] = useState<string | undefined>(() => cachedThumb(src));
  useReglages();
  useEffect(() => {
    let alive = true;
    if (!cachedThumb(src)) setUrl(undefined);
    loadThumb(src).then(
      (u) => alive && setUrl(u),
      () => {},
    );
    return () => {
      alive = false;
    };
  }, [src]);
  return (
    <span className="mini-thumb">
      {url && <img src={url} alt="" style={{ filter: filtreDe(src) }} />}
    </span>
  );
}
