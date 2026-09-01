// SPDX-License-Identifier: GPL-3.0-or-later
import { EditorView } from "@codemirror/view";

export const replaEditorTheme = EditorView.theme({
  "&": {
    height: "100%",
    color: "var(--text-primary)",
    backgroundColor: "transparent",
    fontSize: "13px",
  },
  "&.cm-focused": { outline: "none" },
  ".cm-scroller": {
    fontFamily: "var(--font-mono)",
    lineHeight: "1.55",
    overflow: "auto",
  },
  ".cm-content": {
    padding: "14px 18px 40vh",
    caretColor: "var(--accent)",
  },
  ".cm-line": { padding: "0 2px 0 0" },
  ".cm-cursor, .cm-dropCursor": {
    borderLeftColor: "var(--accent)",
    borderLeftWidth: "2px",
  },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection":
    { backgroundColor: "var(--selection)" },
  ".cm-activeLine": { backgroundColor: "var(--editor-active-line)" },
  ".cm-selectionMatch": {
    backgroundColor: "var(--accent-soft-hover)",
    borderRadius: "2px",
  },
  ".cm-searchMatch": {
    backgroundColor: "var(--warning-soft)",
    outline: "1px solid var(--warning)",
    borderRadius: "2px",
  },
  ".cm-searchMatch.cm-searchMatch-selected": {
    backgroundColor: "var(--accent-soft-hover)",
    outline: "1px solid var(--accent)",
  },
  ".cm-specialChar": { color: "var(--danger)" },

  ".cm-panels": {
    backgroundColor: "var(--surface-2)",
    color: "var(--text-primary)",
    borderBottom: "1px solid var(--border)",
  },
  ".cm-panels.cm-panels-top": { borderBottom: "1px solid var(--border)" },
  ".cm-panel.cm-search": {
    padding: "8px 10px",
    display: "flex",
    flexWrap: "wrap",
    alignItems: "center",
    gap: "6px",
    fontFamily: "var(--font-sans)",
    fontSize: "12.5px",
  },
  ".cm-panel.cm-search label": {
    display: "inline-flex",
    alignItems: "center",
    gap: "4px",
    color: "var(--text-secondary)",
  },
  ".cm-textfield": {
    backgroundColor: "var(--surface)",
    color: "var(--text-primary)",
    border: "1px solid var(--border)",
    borderRadius: "5px",
    padding: "3px 7px",
    fontFamily: "var(--font-sans)",
    fontSize: "12.5px",
  },
  ".cm-textfield:focus": { outline: "none", borderColor: "var(--accent)" },
  ".cm-button": {
    backgroundColor: "var(--surface)",
    backgroundImage: "none",
    color: "var(--text-primary)",
    border: "1px solid var(--border)",
    borderRadius: "5px",
    padding: "3px 8px",
    fontFamily: "var(--font-sans)",
    fontSize: "12.5px",
  },
  ".cm-button:hover": { backgroundColor: "var(--surface-3)" },
  ".cm-panel.cm-search [name='close']": {
    color: "var(--text-muted)",
    fontSize: "16px",
    padding: "0 4px",
  },
});
