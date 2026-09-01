// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useRef } from "react";
import { EditorState, Prec } from "@codemirror/state";
import {
  EditorView,
  crosshairCursor,
  drawSelection,
  dropCursor,
  highlightActiveLine,
  highlightSpecialChars,
  keymap,
  rectangularSelection,
} from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import {
  highlightSelectionMatches,
  openSearchPanel,
  search,
  searchKeymap,
} from "@codemirror/search";
import { indentUnit } from "@codemirror/language";
import { replaEditorTheme } from "./theme";

export interface EditorHandle {
  getValue: () => string;
  focus: () => void;
}

interface CodeEditorProps {
  initialValue: string;
  onDirtyChange: (dirty: boolean) => void;
  onSave: () => void;
  onEscape: () => void;
  handleRef: { current: EditorHandle | null };
}

export function CodeEditor({
  initialValue,
  onDirtyChange,
  onSave,
  onEscape,
  handleRef,
}: CodeEditorProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const statusRef = useRef<HTMLSpanElement>(null);
  const dirtyRef = useRef(false);

  const callbacks = useRef({ onDirtyChange, onSave, onEscape });
  callbacks.current = { onDirtyChange, onSave, onEscape };

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;

    const writeStatus = (state: EditorState) => {
      const node = statusRef.current;
      if (!node) return;
      const head = state.selection.main.head;
      const line = state.doc.lineAt(head);
      const column = head - line.from + 1;
      const chars = state.doc.length;
      const selected = state.selection.main.to - state.selection.main.from;
      node.textContent =
        `Ln ${line.number}, Col ${column}` +
        (selected > 0 ? `  ·  ${selected.toLocaleString()} selected` : "") +
        `  ·  ${chars.toLocaleString()} character${chars === 1 ? "" : "s"}` +
        `  ·  ${state.doc.lines.toLocaleString()} line${state.doc.lines === 1 ? "" : "s"}`;
    };

    const view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: initialValue,
        extensions: [
          history(),
          drawSelection(),
          dropCursor(),
          EditorState.allowMultipleSelections.of(true),
          rectangularSelection(),
          crosshairCursor(),
          highlightSpecialChars(),
          highlightActiveLine(),
          highlightSelectionMatches(),
          search({ top: true }),
          EditorView.lineWrapping,
          EditorState.tabSize.of(4),
          indentUnit.of("    "),
          EditorView.contentAttributes.of({
            "aria-label": "Snippet content",
            spellcheck: "false",
            autocorrect: "off",
            autocapitalize: "off",
          }),
          keymap.of([
            ...searchKeymap,
            ...defaultKeymap,
            ...historyKeymap,
            indentWithTab,
          ]),

          Prec.lowest(
            keymap.of([
              {
                key: "Mod-s",
                preventDefault: true,
                run: () => {
                  callbacks.current.onSave();
                  return true;
                },
              },
              {
                key: "Mod-h",
                preventDefault: true,
                run: (target) => openSearchPanel(target),
              },
              {
                key: "Escape",
                run: () => {
                  callbacks.current.onEscape();
                  return true;
                },
              },
            ]),
          ),
          replaEditorTheme,
          EditorView.updateListener.of((update) => {
            if (update.docChanged && !dirtyRef.current) {
              dirtyRef.current = true;
              callbacks.current.onDirtyChange(true);
            }
            if (update.docChanged || update.selectionSet) {
              writeStatus(update.state);
            }
          }),
        ],
      }),
    });

    writeStatus(view.state);
    handleRef.current = {
      getValue: () => view.state.doc.toString(),
      focus: () => view.focus(),
    };
    view.focus();

    return () => {
      handleRef.current = null;
      view.destroy();
    };

    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div
        ref={hostRef}
        className="min-h-0 flex-1 overflow-hidden rounded-[8px] border border-border bg-surface"
      />
      <div className="flex h-7 shrink-0 items-center justify-between px-1 pt-1.5">
        <span ref={statusRef} className="font-mono text-[11px] text-muted" />
        <span className="text-[11px] text-muted">
          Ctrl F find · Ctrl H replace · Ctrl S save
        </span>
      </div>
    </div>
  );
}
