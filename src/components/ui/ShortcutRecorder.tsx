// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useState } from "react";
import { Kbd } from "./Kbd";
import { Button } from "./Button";

interface ShortcutRecorderProps {
  value: string;
  onChange: (accelerator: string) => void;
}

export function ShortcutRecorder({ value, onChange }: ShortcutRecorderProps) {
  const [recording, setRecording] = useState(false);

  useEffect(() => {
    if (!recording) return;

    const onKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();

      if (event.key === "Escape") {
        setRecording(false);
        return;
      }

      const key = keyName(event.code);
      if (!key) return;

      const parts: string[] = [];
      if (event.ctrlKey || event.metaKey) parts.push("CommandOrControl");
      if (event.altKey) parts.push("Alt");
      if (event.shiftKey) parts.push("Shift");
      if (parts.length === 0) return;

      parts.push(key);
      setRecording(false);
      onChange(parts.join("+"));
    };

    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [recording, onChange]);

  if (recording) {
    return (
      <div className="flex items-center gap-2">
        <span className="text-[12.5px] text-accent">Press a combination…</span>
        <Button size="sm" onClick={() => setRecording(false)}>
          Cancel
        </Button>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2">
      <Kbd keys={humanise(value)} />
      <Button size="sm" onClick={() => setRecording(true)}>
        Change
      </Button>
    </div>
  );
}

export function humanise(accelerator: string): string {
  return accelerator
    .split("+")
    .map((part) => (part === "CommandOrControl" || part === "CmdOrCtrl" ? "Ctrl" : part))
    .join(" ");
}

function keyName(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code;
  switch (code) {
    case "Space":
      return "Space";
    case "Enter":
    case "NumpadEnter":
      return "Enter";
    case "Tab":
      return "Tab";
    case "Backquote":
      return "`";
    case "Minus":
      return "-";
    case "Equal":
      return "=";
    case "BracketLeft":
      return "[";
    case "BracketRight":
      return "]";
    case "Backslash":
      return "\\";
    case "Semicolon":
      return ";";
    case "Quote":
      return "'";
    case "Comma":
      return ",";
    case "Period":
      return ".";
    case "Slash":
      return "/";
    case "ArrowUp":
      return "Up";
    case "ArrowDown":
      return "Down";
    case "ArrowLeft":
      return "Left";
    case "ArrowRight":
      return "Right";
    default:
      return null;
  }
}
