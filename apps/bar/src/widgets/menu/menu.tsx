import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent,
  type ReactNode,
} from "react";

import {
  commands,
  events,
  type MenuEntry,
  type MenuOpen,
} from "@/shared/bindings";

type MenuLevel = "root" | "submenu";
type SubmenuSide = "left" | "right";

type MenuLayout = {
  rootWidth: number;
  rootHeight: number;
  submenuWidth: number;
  submenuHeight: number;
};

const PANEL_CLASS =
  "box-border w-max min-w-[248px] max-w-[480px] shrink-0 " +
  "overflow-visible rounded-[var(--window-radius,8px)] " +
  "border border-[var(--color-panel-separator)] " +
  "bg-[var(--color-panel)] p-[6px] text-[var(--color-panel-fg)] " +
  "font-[var(--font-bar)] text-[13px] " +
  "shadow-[0_8px_28px_rgba(0,0,0,0.24)]";

const ITEM_CLASS =
  "flex min-h-[30px] w-full items-center gap-2 rounded-[5px] px-2 py-0 " +
  "text-left text-[13px] text-[var(--color-panel-fg)] outline-none " +
  "transition-colors duration-75 hover:bg-white/[0.10] " +
  "focus:bg-white/[0.14] focus-visible:bg-white/[0.14] " +
  "disabled:cursor-default disabled:text-[var(--color-panel-fg-muted)]";

const MUTED_TEXT_CLASS = "text-xs tracking-normal text-[var(--color-panel-fg-muted)]";

const PANEL_STYLE: CSSProperties = {
  width: "max-content",
  minWidth: 248,
  maxWidth: 480,
  maxHeight: "none",
};

const EMPTY_LAYOUT: MenuLayout = {
  rootWidth: 248,
  rootHeight: 0,
  submenuWidth: 0,
  submenuHeight: 0,
};

function firstNavigableIndex(entries: MenuEntry[]): number {
  return entries.findIndex(
    (entry) => entry.kind !== "separator" && entry.enabled,
  );
}

function nextNavigableIndex(
  entries: MenuEntry[],
  current: number,
  direction: 1 | -1,
): number {
  if (entries.length === 0) return 0;

  for (let step = 1; step <= entries.length; step += 1) {
    const index = (current + direction * step + entries.length) % entries.length;
    const entry = entries[index];
    if (entry && entry.kind !== "separator" && entry.enabled) return index;
  }

  return current;
}

function isActionEntry(entry: MenuEntry): boolean {
  return (
    entry.kind !== "separator" &&
    entry.kind !== "submenu" &&
    Boolean(entry.id)
  );
}

function Indicator({ entry }: { entry: MenuEntry }): ReactNode {
  if (entry.kind === "checkbox") {
    return entry.checked ? <span aria-hidden="true">✓</span> : null;
  }

  if (entry.kind === "radio") {
    return entry.checked ? <span aria-hidden="true">●</span> : null;
  }

  if (entry.icon) {
    return (
      <img
        src={entry.icon}
        alt=""
        className="size-4 object-contain"
        draggable={false}
      />
    );
  }

  return null;
}

export default function MenuWindow() {
  const [menu, setMenu] = useState<MenuOpen | null>(null);
  const [openSubmenu, setOpenSubmenu] = useState<number | null>(null);
  const [submenuSide, setSubmenuSide] = useState<SubmenuSide>("right");
  const [submenuTop, setSubmenuTop] = useState(0);
  const [rootActiveIndex, setRootActiveIndex] = useState(0);
  const [submenuActiveIndex, setSubmenuActiveIndex] = useState(0);
  const [pendingEntryId, setPendingEntryId] = useState<string | null>(null);
  const [layout, setLayout] = useState<MenuLayout>(EMPTY_LAYOUT);

  const rootRef = useRef<HTMLDivElement>(null);
  const submenuRef = useRef<HTMLDivElement>(null);
  const submenuTriggerRefs = useRef(new Map<number, HTMLDivElement>());
  const itemRefs = useRef(new Map<string, HTMLButtonElement>());
  const closingRef = useRef(false);
  const activatingRef = useRef(false);

  const closeMenu = useCallback(() => {
    if (closingRef.current) return;
    closingRef.current = true;

    void commands.closeMenu().catch((error) => {
      closingRef.current = false;
      console.error("Could not close the menu:", error);
    });
  }, []);

  const activate = useCallback(
    async (entry: MenuEntry) => {
      if (
        !menu ||
        !entry.enabled ||
        !entry.id ||
        entry.kind === "separator" ||
        entry.kind === "submenu" ||
        activatingRef.current
      ) {
        return;
      }

      activatingRef.current = true;
      setPendingEntryId(entry.id);

      try {
        await commands.menuItemClicked(menu.itemId, entry.id);
      } catch (error) {
        console.error("Could not activate menu item:", error);
      } finally {
        activatingRef.current = false;
        setPendingEntryId(null);
      }
    },
    [menu],
  );

  // Subscribe before looking up the current menu so an open event cannot be missed.
  useEffect(() => {
    let cancelled = false;
    let receivedMenuOpenEvent = false;
    let unlisten: Array<() => void> = [];

    async function subscribe() {
      try {
        const stops = await Promise.all([
          events.menuOpen.listen(({ payload }) => {
            receivedMenuOpenEvent = true;
            closingRef.current = false;
            activatingRef.current = false;
            setPendingEntryId(null);
            setMenu(payload);
            setOpenSubmenu(null);
            setSubmenuSide(payload.anchor.submenuSide);
            setSubmenuTop(0);
            setRootActiveIndex(Math.max(0, firstNavigableIndex(payload.menu)));
            setSubmenuActiveIndex(0);
          }),
          events.menuChanged.listen(({ payload }) => {
            setMenu((current) => {
              if (!current || current.itemId !== payload.itemId) return current;
              return { ...current, menu: payload.menu };
            });
            setOpenSubmenu(null);
            setRootActiveIndex(Math.max(0, firstNavigableIndex(payload.menu)));
            setSubmenuActiveIndex(0);
          }),
          events.menuClose.listen(({ payload }) => {
            setMenu((current) =>
              current?.itemId === payload.itemId ? null : current,
            );
            setOpenSubmenu(null);
            setPendingEntryId(null);
            closingRef.current = false;
            activatingRef.current = false;
          }),
        ]);

        if (cancelled) {
          stops.forEach((stop) => stop());
          return;
        }

        unlisten = stops;

        // A delayed initial lookup must not overwrite a newer menu-open event.
        const current = await commands.getOpenMenu();
        if (!cancelled && !receivedMenuOpenEvent && current) {
          closingRef.current = false;
          activatingRef.current = false;
          setMenu(current);
          setSubmenuSide(current.anchor.submenuSide);
          setRootActiveIndex(Math.max(0, firstNavigableIndex(current.menu)));
        }
      } catch (error) {
        console.error("Could not initialize the menu window:", error);
      }
    }

    void subscribe();

    return () => {
      cancelled = true;
      unlisten.forEach((stop) => stop());
    };
  }, []);

  // Put keyboard focus inside the active menu level whenever it opens or changes selection.
  useEffect(() => {
    if (!menu) return;

    const frame = requestAnimationFrame(() => {
      const level: MenuLevel = openSubmenu === null ? "root" : "submenu";
      const index = level === "root" ? rootActiveIndex : submenuActiveIndex;
      itemRefs.current.get(`${level}:${index}`)?.focus({ preventScroll: true });
      if (!itemRefs.current.has(`${level}:${index}`)) {
        (level === "root" ? rootRef.current : submenuRef.current)?.focus({
          preventScroll: true,
        });
      }
    });

    return () => cancelAnimationFrame(frame);
  }, [menu?.itemId, openSubmenu, rootActiveIndex, submenuActiveIndex]);

  // Measure the actual DOM panels and tell Rust the complete window size.
  // The submenu is a sibling panel, not a Radix portal, so its position is predictable.
  useLayoutEffect(() => {
    if (!menu) return;

    let frame: number | null = null;
    let requestVersion = 0;
    let disposed = false;

    const measure = () => {
      if (frame !== null) cancelAnimationFrame(frame);

      frame = requestAnimationFrame(() => {
        frame = null;
        if (disposed) return;

        const root = rootRef.current;
        if (!root) return;

        const rootWidth = Math.ceil(
          Math.max(root.getBoundingClientRect().width, root.scrollWidth),
        );
        const rootHeight = Math.ceil(
          Math.max(root.getBoundingClientRect().height, root.scrollHeight),
        );

        const submenu = submenuRef.current;
        const submenuWidth = submenu
          ? Math.ceil(
              Math.max(submenu.getBoundingClientRect().width, submenu.scrollWidth),
            )
          : 0;
        const submenuHeight = submenu
          ? Math.ceil(
              Math.max(submenu.getBoundingClientRect().height, submenu.scrollHeight),
            )
          : 0;

        let nextSubmenuTop = 0;
        if (submenu && openSubmenu !== null) {
          const trigger = submenuTriggerRefs.current.get(openSubmenu);
          if (trigger) {
            const triggerTop =
              trigger.getBoundingClientRect().top - root.getBoundingClientRect().top;
            nextSubmenuTop = Math.max(
              0,
              Math.min(triggerTop, Math.max(0, rootHeight - submenuHeight)),
            );
          }
        }

        const nextLayout: MenuLayout = {
          rootWidth,
          rootHeight,
          submenuWidth,
          submenuHeight,
        };

        setLayout((current) =>
          current.rootWidth === nextLayout.rootWidth &&
          current.rootHeight === nextLayout.rootHeight &&
          current.submenuWidth === nextLayout.submenuWidth &&
          current.submenuHeight === nextLayout.submenuHeight
            ? current
            : nextLayout,
        );
        setSubmenuTop((current) =>
          current === nextSubmenuTop ? current : nextSubmenuTop,
        );

        const totalWidth = rootWidth + (submenu ? submenuWidth + 4 : 0);
        const totalHeight = Math.max(rootHeight, nextSubmenuTop + submenuHeight);
        const currentRequest = ++requestVersion;

        void commands
          .menuResized(totalWidth, totalHeight, rootWidth)
          .then((placement) => {
            if (disposed || currentRequest !== requestVersion) return;
            setSubmenuSide(placement.submenuSide);
          })
          .catch((error) => {
            if (!disposed && currentRequest === requestVersion) {
              console.error("Could not resize the menu:", error);
            }
          });
      });
    };

    const observer = new ResizeObserver(measure);
    if (rootRef.current) observer.observe(rootRef.current);
    if (submenuRef.current) observer.observe(submenuRef.current);
    measure();

    return () => {
      disposed = true;
      requestVersion += 1;
      observer.disconnect();
      if (frame !== null) cancelAnimationFrame(frame);
    };
  }, [menu, openSubmenu]);

  function handleKeyDown(event: KeyboardEvent<HTMLDivElement>) {
    if (!menu) return;

    const inSubmenu = openSubmenu !== null;
    const entries = inSubmenu
      ? menu.menu[openSubmenu]?.submenu ?? []
      : menu.menu;
    const activeIndex = inSubmenu ? submenuActiveIndex : rootActiveIndex;
    const activeEntry = entries[activeIndex];
    const setActive = inSubmenu ? setSubmenuActiveIndex : setRootActiveIndex;

    switch (event.key) {
      case "Escape":
        event.preventDefault();
        if (inSubmenu) {
          setOpenSubmenu(null);
        } else {
          closeMenu();
        }
        break;

      case "Tab":
        closeMenu();
        break;

      case "ArrowDown":
        event.preventDefault();
        setActive(nextNavigableIndex(entries, activeIndex, 1));
        break;

      case "ArrowUp":
        event.preventDefault();
        setActive(nextNavigableIndex(entries, activeIndex, -1));
        break;

      case "Home": {
        event.preventDefault();
        const first = firstNavigableIndex(entries);
        if (first >= 0) setActive(first);
        break;
      }

      case "End": {
        event.preventDefault();
        for (let index = entries.length - 1; index >= 0; index -= 1) {
          const entry = entries[index];
          if (entry && entry.kind !== "separator" && entry.enabled) {
            setActive(index);
            break;
          }
        }
        break;
      }

      case "ArrowRight":
        if (!inSubmenu && activeEntry?.kind === "submenu" && activeEntry.enabled) {
          event.preventDefault();
          setOpenSubmenu(activeIndex);
          setSubmenuActiveIndex(Math.max(0, firstNavigableIndex(activeEntry.submenu)));
        }
        break;

      case "ArrowLeft":
        if (inSubmenu) {
          event.preventDefault();
          setOpenSubmenu(null);
        }
        break;

      case "Enter":
      case " ":
        event.preventDefault();
        if (activeEntry?.kind === "submenu" && !inSubmenu && activeEntry.enabled) {
          setOpenSubmenu(activeIndex);
          setSubmenuActiveIndex(Math.max(0, firstNavigableIndex(activeEntry.submenu)));
        } else if (activeEntry && isActionEntry(activeEntry)) {
          void activate(activeEntry);
        }
        break;
    }
  }

  function renderEntryContent(entry: MenuEntry): ReactNode {
    return (
      <>
        <span className="grid size-[18px] shrink-0 place-items-center text-[12px]">
          <Indicator entry={entry} />
        </span>
        <span className="min-w-0 flex-1 truncate whitespace-nowrap">
          {entry.label}
        </span>
        {entry.accelerator && (
          <span className={`ml-auto shrink-0 whitespace-nowrap pl-4 ${MUTED_TEXT_CLASS}`}>
            {entry.accelerator}
          </span>
        )}
      </>
    );
  }

  function renderEntries(entries: MenuEntry[], level: MenuLevel): ReactNode[] {
    return entries.map((entry, index) => {
      const key = entry.id ?? `${level}-${index}`;

      if (entry.kind === "separator") {
        return (
          <div
            key={`separator-${level}-${index}`}
            role="separator"
            className="mx-[6px] my-1 h-px bg-[var(--color-panel-separator)]"
          />
        );
      }

      if (entry.kind === "submenu" && level === "root") {
        const isOpen = openSubmenu === index;
        return (
          <div
            key={key}
            ref={(node) => {
              if (node) submenuTriggerRefs.current.set(index, node);
              else submenuTriggerRefs.current.delete(index);
            }}
            className="relative"
            onMouseEnter={() => {
              setRootActiveIndex(index);
              setOpenSubmenu(entry.enabled ? index : null);
              if (entry.enabled) {
                setSubmenuActiveIndex(Math.max(0, firstNavigableIndex(entry.submenu)));
              }
            }}
          >
            <button
              ref={(node) => {
                const refKey = `root:${index}`;
                if (node) itemRefs.current.set(refKey, node);
                else itemRefs.current.delete(refKey);
              }}
              type="button"
              role="menuitem"
              aria-haspopup="menu"
              aria-expanded={isOpen}
              disabled={!entry.enabled}
              tabIndex={rootActiveIndex === index ? 0 : -1}
              className={ITEM_CLASS}
              onFocus={() => setRootActiveIndex(index)}
              onClick={() => {
                if (!entry.enabled) return;
                setOpenSubmenu((current) => (current === index ? null : index));
                setSubmenuActiveIndex(Math.max(0, firstNavigableIndex(entry.submenu)));
              }}
            >
              {renderEntryContent(entry)}
              <span aria-hidden="true" className="ml-2 shrink-0 text-[17px] leading-none">
                ›
              </span>
            </button>
          </div>
        );
      }

      const disabled =
        !entry.enabled ||
        !entry.id ||
        pendingEntryId !== null;
      const activeIndex = level === "root" ? rootActiveIndex : submenuActiveIndex;
      const setActiveIndex =
        level === "root" ? setRootActiveIndex : setSubmenuActiveIndex;
      const role =
        entry.kind === "checkbox"
          ? "menuitemcheckbox"
          : entry.kind === "radio"
            ? "menuitemradio"
            : "menuitem";

      return (
        <button
          key={key}
          ref={(node) => {
            const refKey = `${level}:${index}`;
            if (node) itemRefs.current.set(refKey, node);
            else itemRefs.current.delete(refKey);
          }}
          type="button"
          role={role}
          aria-checked={entry.kind === "checkbox" || entry.kind === "radio" ? entry.checked : undefined}
          disabled={disabled}
          tabIndex={activeIndex === index ? 0 : -1}
          className={ITEM_CLASS}
          onFocus={() => setActiveIndex(index)}
          onMouseEnter={() => {
            setActiveIndex(index);
            if (level === "root") setOpenSubmenu(null);
          }}
          onClick={() => void activate(entry)}
        >
          {renderEntryContent(entry)}
          {pendingEntryId === entry.id && (
            <span aria-hidden="true" className="ml-1 size-3 shrink-0 animate-spin rounded-full border border-current border-r-transparent" />
          )}
        </button>
      );
    });
  }

  if (!menu) return null;

  const submenuEntries =
    openSubmenu === null ? null : menu.menu[openSubmenu]?.submenu ?? [];
  const rootLeft =
    openSubmenu !== null && submenuSide === "left"
      ? layout.submenuWidth + 4
      : 0;
  const submenuLeft =
    submenuSide === "right" ? layout.rootWidth + 4 : 0;

  const rootStyle: CSSProperties = {
    ...PANEL_STYLE,
    position: "absolute",
    top: 0,
    left: rootLeft,
    zIndex: 1,
    pointerEvents: "auto",
  };
  const submenuStyle: CSSProperties = {
    ...PANEL_STYLE,
    position: "absolute",
    top: submenuTop,
    left: submenuLeft,
    zIndex: 2,
    pointerEvents: "auto",
  };

  return (
    <div
      className="fixed inset-0 overflow-visible"
      style={{ pointerEvents: "none" }}
      onKeyDown={handleKeyDown}
    >
      <div
        ref={rootRef}
        role="menu"
        aria-label="Barista menu"
        tabIndex={-1}
        className={PANEL_CLASS}
        style={rootStyle}
      >
        {renderEntries(menu.menu, "root")}
      </div>

      {submenuEntries && (
        <div
          ref={submenuRef}
          role="menu"
          aria-label="Submenu"
          tabIndex={-1}
          className={PANEL_CLASS}
          style={submenuStyle}
        >
          {renderEntries(submenuEntries, "submenu")}
        </div>
      )}
    </div>
  );
}
