"use client";

import { useTheme } from "next-themes";
import { Moon, Palette, Sun, SunMoon } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { useTenantTheme } from "@/components/skycot/ThemeProvider";

/**
 * Two controls: light/dark/system + tenant brand. Renders inline
 * for the demo page; production usage drops it into the operator
 * console's settings panel.
 */
export function ThemePicker() {
  const { theme: mode, setTheme: setMode } = useTheme();
  const { theme, setThemeById, availableThemes } = useTenantTheme();

  return (
    <div className="flex flex-wrap items-center gap-3">
      <div className="inline-flex rounded-[var(--radius-md)] border border-[var(--border)] p-0.5">
        <ModeButton current={mode} value="light" onClick={() => setMode("light")} icon={<Sun />} label="Light" />
        <ModeButton current={mode} value="system" onClick={() => setMode("system")} icon={<SunMoon />} label="System" />
        <ModeButton current={mode} value="dark" onClick={() => setMode("dark")} icon={<Moon />} label="Dark" />
      </div>

      <div className="inline-flex items-center gap-2 text-xs text-[var(--muted-foreground)]">
        <Palette className="size-4" />
        <label htmlFor="tenant-theme" className="sr-only">Tenant theme</label>
        <select
          id="tenant-theme"
          className="rounded-[var(--radius-md)] border border-[var(--input)] bg-[var(--background)] px-2 py-1.5 text-xs text-[var(--foreground)]"
          value={theme.id}
          onChange={(e) => setThemeById(e.target.value)}
        >
          {availableThemes.map((t) => (
            <option key={t.id} value={t.id}>
              {t.label}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}

function ModeButton({
  current,
  value,
  onClick,
  icon,
  label,
}: {
  current: string | undefined;
  value: string;
  onClick: () => void;
  icon: React.ReactNode;
  label: string;
}) {
  const active = current === value;
  return (
    <Button
      variant={active ? "primary" : "ghost"}
      size="sm"
      onClick={onClick}
      aria-pressed={active}
      title={label}
      data-testid={`mode-${value}`}
    >
      {icon}
      <span className="sr-only">{label}</span>
    </Button>
  );
}
