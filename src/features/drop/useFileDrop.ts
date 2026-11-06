// Native drag & drop: Tauri gives us real file paths (the browser API would not).
import { useEffect, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";

export function useFileDrop(onDrop: (paths: string[]) => void) {
  const [hovering, setHovering] = useState(false);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;

    getCurrentWebview()
      .onDragDropEvent((event) => {
        const p = event.payload;
        if (p.type === "enter" || p.type === "over") setHovering(true);
        else if (p.type === "leave") setHovering(false);
        else if (p.type === "drop") {
          setHovering(false);
          if (p.paths.length) onDrop(p.paths);
        }
      })
      .then((fn) => {
        if (disposed) fn();
        else unlisten = fn;
      });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [onDrop]);

  return { hovering };
}
