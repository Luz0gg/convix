// Accessible overlay used by the settings drawer and the module prompt:
// role="dialog", Escape closes, focus moves inside and comes back where it was,
// Tab stays inside while it is open.
import { useEffect, useRef, type ReactNode } from "react";

const FOCUSABLE = 'button:not([disabled]), [href], input:not([disabled]), select, textarea, [tabindex]:not([tabindex="-1"])';

export function Dialog({
  title,
  onClose,
  children,
  variant = "center",
  closeOnBackdrop = true,
}: {
  /** Announced by screen readers (the visible heading lives in `children`). */
  title: string;
  /** Undefined = cannot be dismissed right now (e.g. a download is running). */
  onClose?: () => void;
  children: ReactNode;
  variant?: "center" | "drawer";
  closeOnBackdrop?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;

  useEffect(() => {
    const previous = document.activeElement as HTMLElement | null;
    const panel = ref.current;
    panel?.querySelector<HTMLElement>(FOCUSABLE)?.focus();

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" && onCloseRef.current) {
        e.stopPropagation();
        onCloseRef.current();
      }
      if (e.key === "Tab" && panel) {
        const items = [...panel.querySelectorAll<HTMLElement>(FOCUSABLE)];
        if (items.length === 0) return;
        const [first, last] = [items[0], items[items.length - 1]];
        if (e.shiftKey && document.activeElement === first) {
          e.preventDefault();
          last.focus();
        } else if (!e.shiftKey && document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };
    document.addEventListener("keydown", onKey, true);
    return () => {
      document.removeEventListener("keydown", onKey, true);
      previous?.focus?.();
    };
  }, []);

  const drawer = variant === "drawer";
  return (
    <div
      className={`fixed inset-0 z-30 flex bg-black/50 ${drawer ? "justify-end" : "items-center justify-center p-6"}`}
      onClick={() => closeOnBackdrop && onClose?.()}
    >
      <div
        ref={ref}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        onClick={(e) => e.stopPropagation()}
        className={
          drawer
            ? "h-full w-full max-w-md space-y-6 overflow-y-auto border-l border-border bg-surface p-6"
            : "w-full max-w-md space-y-4 rounded-xl border border-border bg-surface p-6 shadow-xl"
        }
      >
        {children}
      </div>
    </div>
  );
}
