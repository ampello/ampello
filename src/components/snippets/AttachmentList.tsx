// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import {
  AlertTriangle,
  ChevronLeft,
  ChevronRight,
  File,
  FileArchive,
  FileAudio,
  FileSpreadsheet,
  FileText,
  FileVideo,
  Paperclip,
  Plus,
  Upload,
  X,
} from "lucide-react";
import { Button } from "@/components/ui/Button";
import { SegmentedControl } from "@/components/ui/SegmentedControl";
import { Switch } from "@/components/ui/Switch";
import { Tooltip } from "@/components/ui/Tooltip";
import * as ipc from "@/lib/ipc";
import { cn } from "@/lib/cn";
import { useFileDrop } from "@/lib/fileDrop";
import { reportError, useToastStore } from "@/stores/toastStore";
import type { Attachment, Snippet } from "@/lib/types";

export function AttachmentList({
  snippet,
  onChange,
}: {
  snippet: Snippet | null;
  onChange: (snippet: Snippet) => void;
}) {
  const [open, setOpen] = useState(false);
  const [busy, setBusy] = useState(false);
  const pushToast = useToastStore((s) => s.push);

  const dragging = useFileDrop((paths) => {
    if (!snippet) {
      pushToast("info", "Save this snippet before attaching files to it.");
      return;
    }
    void run(() => ipc.addAttachments(snippet.id, paths), true);
  });

  if (!snippet) {
    return (
      <section
        className={cn(
          "mt-3 flex shrink-0 items-center gap-2 rounded-[10px] border border-dashed px-3 py-2",
          dragging ? "border-border-strong bg-surface-2" : "border-border",
        )}
      >
        <Paperclip size={13} strokeWidth={1.75} className="text-muted" />
        <p className="text-[12px] text-muted">
          Save this snippet before attaching files to it.
        </p>
      </section>
    );
  }

  const attachments = snippet.attachments;

  const run = async (action: () => Promise<Snippet | null>, reveal = false) => {
    if (busy) return;
    setBusy(true);
    try {
      const updated = await action();
      if (updated) {
        onChange(updated);

        if (reveal && updated.attachments.length > 0) setOpen(true);
      }
    } catch (error) {
      reportError(error);
    } finally {
      setBusy(false);
    }
  };

  const move = (index: number, delta: number) => {
    const next = [...attachments];
    const target = index + delta;
    if (target < 0 || target >= next.length) return;
    [next[index], next[target]] = [next[target], next[index]];
    void run(() =>
      ipc.reorderAttachments(
        snippet.id,
        next.map((attachment) => attachment.id),
      ),
    );
  };

  const totalBytes = attachments.reduce((total, a) => total + a.sizeBytes, 0);
  const panelId = `attachments-${snippet.id}`;

  return (
    <section className="mt-3 shrink-0">
      {dragging ? (
        <div
          aria-hidden="true"
          className="motion-fade fixed inset-0 z-50 flex items-center justify-center"

          style={{ backgroundColor: "var(--overlay)" }}
        >
          <div className="motion-pop flex flex-col items-center gap-2 rounded-[12px] border border-dashed border-accent bg-surface px-8 py-6 shadow-[var(--shadow-lg)]">
            <Upload size={22} strokeWidth={1.5} className="text-accent" />
            <p className="text-[13.5px] font-medium text-primary">
              Drop to attach to {snippet.trigger}
            </p>
            <p className="text-[12px] text-muted">Any kind of file.</p>
          </div>
        </div>
      ) : null}

      <div
        className={cn(
          "overflow-hidden rounded-[10px] border bg-surface transition-colors duration-100",
          dragging ? "border-accent" : "border-border",
        )}
      >
        <div className="flex h-9 items-center gap-2 pl-1 pr-1.5">
          <button
            type="button"
            aria-expanded={open}
            aria-controls={panelId}
            onClick={() => setOpen((value) => !value)}
            className={cn(
              "flex h-7 min-w-0 flex-1 items-center gap-2 rounded-[6px] px-1.5 text-left",
              "transition-colors duration-100 hover:bg-surface-2",
              "focus-visible:outline-2 focus-visible:outline-accent",
            )}
          >
            <ChevronRight
              size={13}
              strokeWidth={2}
              className={cn(
                "shrink-0 text-muted transition-transform duration-150 ease-[var(--ease-out)]",
                open && "rotate-90",
              )}
            />
            <Paperclip size={12} strokeWidth={1.75} className="shrink-0 text-muted" />
            <span className="shrink-0 text-[12px] font-medium text-secondary">
              Attachments
            </span>
            <span className="shrink-0 text-[11.5px] text-muted">
              {attachments.length === 0
                ? "None yet"
                : `${attachments.length} file${attachments.length === 1 ? "" : "s"} · ${formatSize(totalBytes)}`}
            </span>
            {!open && attachments.length > 0 ? (
              <span className="ml-1 flex min-w-0 items-center gap-1">
                {attachments.slice(0, 4).map((attachment) => (
                  <Chip key={attachment.id} attachment={attachment} />
                ))}
                {attachments.length > 4 ? (
                  <span className="text-[10.5px] tabular-nums text-muted">
                    +{attachments.length - 4}
                  </span>
                ) : null}
              </span>
            ) : null}
          </button>

          <Button
            onClick={() => void run(() => ipc.pickAttachments(snippet.id), true)}
            disabled={busy}
          >
            <Plus size={13} strokeWidth={2} />
            Add files
          </Button>
        </div>

        {open ? (
          <div id={panelId} className="border-t border-border">
            {attachments.length === 0 ? (
              <p className="px-3 py-2.5 text-[12.5px] text-muted">
                Any kind of file: an image, a PDF, a document, an archive. They
                are handed to whatever you are typing in, which attaches or
                uploads them if it can.
              </p>
            ) : (
              <>
                <div className="flex gap-2 overflow-x-auto p-2.5">
                  {attachments.map((attachment, index) => (
                    <Tile
                      key={attachment.id}
                      attachment={attachment}
                      index={index}
                      last={index === attachments.length - 1}
                      busy={busy}
                      onMove={(delta) => move(index, delta)}
                      onRemove={() => void run(() => ipc.removeAttachment(attachment.id))}
                    />
                  ))}
                </div>

                <div className="flex flex-wrap items-center gap-x-6 gap-y-3 border-t border-border px-3 py-2.5">
                  <div className="flex items-center gap-2">
                    <span className="text-[12.5px] text-secondary">Send</span>
                    <SegmentedControl
                      label="Whether the files go in before the text or after it"
                      value={snippet.attachmentsFirst ? "first" : "last"}
                      options={[
                        { value: "first", label: "Files first" },
                        { value: "last", label: "Text first" },
                      ]}
                      onChange={(value) =>
                        void run(() =>
                          ipc.updateSnippet(snippet.id, {
                            attachmentsFirst: value === "first",
                          }),
                        )
                      }
                    />
                  </div>

                  <Tooltip
                    content={
                      "Hands the files over one at a time and waits for each. Slower, but " +
                      "some applications upload in parallel and show the files in whatever " +
                      "order finishes first, which is not the order you set here."
                    }
                  >
                    <div className="flex items-center gap-2">
                      <span className="text-[12.5px] text-secondary">
                        Keep this exact order
                      </span>
                      <Switch
                        label="Hand the files over one at a time"
                        checked={snippet.strictOrder}
                        onChange={(checked) =>
                          void run(() =>
                            ipc.updateSnippet(snippet.id, { strictOrder: checked }),
                          )
                        }
                      />
                    </div>
                  </Tooltip>

                  {snippet.strictOrder ? (
                    <span className="text-[11.5px] text-muted">
                      About {estimateSeconds(attachments.length)}s to insert.
                    </span>
                  ) : null}
                </div>
              </>
            )}
          </div>
        ) : null}
      </div>
    </section>
  );
}

function Chip({ attachment }: { attachment: Attachment }) {
  const preview = usePreview(attachment);

  if (preview) {
    return (
      <img
        src={preview}
        alt=""
        aria-hidden="true"
        draggable={false}
        className="h-[18px] w-[18px] shrink-0 rounded-[3px] border border-border object-cover"
      />
    );
  }
  return (
    <span
      aria-hidden="true"
      className={cn(
        "flex h-[18px] w-[18px] shrink-0 items-center justify-center rounded-[3px] border",
        attachment.present ? "border-border bg-surface-2" : "border-danger/45 bg-danger-soft",
      )}
    >
      <Icon mime={attachment.mime} missing={!attachment.present} size={10} />
    </span>
  );
}

function Tile({
  attachment,
  index,
  last,
  busy,
  onMove,
  onRemove,
}: {
  attachment: Attachment;
  index: number;
  last: boolean;
  busy: boolean;
  onMove: (delta: number) => void;
  onRemove: () => void;
}) {
  const preview = usePreview(attachment);
  const image = Boolean(preview);

  return (
    <div
      className={cn(
        "group/tile relative shrink-0 overflow-hidden rounded-[8px] border bg-surface-2",
        "h-[76px] transition-colors duration-100",
        image ? "w-[76px]" : "w-[188px]",
        attachment.present ? "border-border" : "border-danger/45",
      )}
      title={`${index + 1}. ${attachment.name} — ${formatSize(attachment.sizeBytes)}`}
    >
      {image ? (
        <img
          src={preview ?? undefined}
          alt={attachment.name}
          className="h-full w-full object-cover"
          draggable={false}
        />
      ) : (
        <div className="flex h-full flex-col justify-center gap-1 px-2.5">
          <div className="flex items-center gap-1.5">
            <Icon mime={attachment.mime} missing={!attachment.present} />
            <span className="truncate text-[12px] font-medium text-primary">
              {attachment.name}
            </span>
          </div>
          <span className="pl-[22px] text-[10.5px] uppercase tracking-[0.05em] text-muted">
            {label(attachment)}
          </span>
        </div>
      )}
      <span
        className={cn(
          "absolute left-1 top-1 flex h-4 min-w-4 items-center justify-center rounded-[4px] px-1",
          "text-[10px] font-semibold tabular-nums",
          image
            ? "bg-black/55 text-white backdrop-blur-[2px]"
            : "bg-surface-3 text-secondary",
        )}
      >
        {index + 1}
      </span>

      {!attachment.present ? (
        <span
          className="absolute right-1 top-1 flex items-center gap-1 rounded-[4px] bg-danger-soft px-1 py-0.5 text-[9.5px] font-medium text-danger"
          title="This file is no longer in Ampello's library and will be skipped."
        >
          <AlertTriangle size={9} strokeWidth={2.25} />
          Missing
        </span>
      ) : null}
      <div
        className={cn(
          "absolute inset-x-0 bottom-0 flex items-center justify-center gap-0.5 py-0.5",
          "bg-surface/92 backdrop-blur-[2px] opacity-0 transition-opacity duration-100",
          "group-hover/tile:opacity-100 focus-within:opacity-100",
        )}
      >
        <TileButton
          label={`Move ${attachment.name} earlier`}
          disabled={busy || index === 0}
          onClick={() => onMove(-1)}
        >
          <ChevronLeft size={13} strokeWidth={2} />
        </TileButton>
        <TileButton
          label={`Move ${attachment.name} later`}
          disabled={busy || last}
          onClick={() => onMove(1)}
        >
          <ChevronRight size={13} strokeWidth={2} />
        </TileButton>
        <TileButton
          label={`Remove ${attachment.name}`}
          disabled={busy}
          onClick={onRemove}
          danger
        >
          <X size={13} strokeWidth={2} />
        </TileButton>
      </div>
    </div>
  );
}

function TileButton({
  label,
  disabled,
  danger = false,
  onClick,
  children,
}: {
  label: string;
  disabled: boolean;
  danger?: boolean;
  onClick: () => void;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      disabled={disabled}
      onClick={onClick}
      className={cn(
        "flex h-5 w-5 items-center justify-center rounded-[4px] transition-colors duration-100",
        "text-secondary hover:bg-surface-3 hover:text-primary",
        "focus-visible:outline-2 focus-visible:outline-accent",
        "disabled:pointer-events-none disabled:opacity-35",
        danger && "hover:bg-danger-soft hover:text-danger",
      )}
    >
      {children}
    </button>
  );
}

function usePreview(attachment: Attachment): string | null {
  const [url, setUrl] = useState<string | null>(null);
  const { id, mime, present, sizeBytes } = attachment;

  useEffect(() => {
    if (!present || !previewable(mime) || sizeBytes > MAX_PREVIEW_BYTES) {
      setUrl(null);
      return;
    }

    let revoked = false;
    let current: string | null = null;

    ipc
      .attachmentBytes(id)
      .then((bytes) => {
        if (revoked) return;
        current = URL.createObjectURL(new Blob([toBytes(bytes)], { type: mime }));
        setUrl(current);
      })
      .catch(() => {
        if (!revoked) setUrl(null);
      });

    return () => {
      revoked = true;
      if (current) URL.revokeObjectURL(current);
      setUrl(null);
    };
  }, [id, mime, present, sizeBytes]);

  return url;
}

function toBytes(raw: ArrayBuffer | Uint8Array | number[]): BlobPart {
  if (raw instanceof ArrayBuffer) return raw;
  return Uint8Array.from(raw);
}

const PREVIEWABLE = [
  "image/png",
  "image/jpeg",
  "image/gif",
  "image/webp",
  "image/bmp",
  "image/svg+xml",
];
const MAX_PREVIEW_BYTES = 8 * 1024 * 1024;

function previewable(mime: string): boolean {
  return PREVIEWABLE.includes(mime);
}

function label(attachment: Attachment): string {
  const extension = attachment.name.includes(".")
    ? attachment.name.split(".").pop()?.toUpperCase()
    : null;
  return `${extension ?? "File"} · ${formatSize(attachment.sizeBytes)}`;
}

function estimateSeconds(count: number): number {
  return Math.max(1, Math.round((count * 530) / 100) / 10);
}

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function Icon({
  mime,
  missing,
  size = 15,
}: {
  mime: string;
  missing: boolean;
  size?: number;
}) {
  const props = { size, strokeWidth: 1.75 as const };

  if (missing) return <AlertTriangle {...props} className="shrink-0 text-danger" />;

  const className = "shrink-0 text-muted";
  if (mime.startsWith("audio/")) return <FileAudio {...props} className={className} />;
  if (mime.startsWith("video/")) return <FileVideo {...props} className={className} />;
  if (mime === "application/zip") return <FileArchive {...props} className={className} />;
  if (mime.includes("spreadsheet") || mime === "text/csv" || mime.includes("excel")) {
    return <FileSpreadsheet {...props} className={className} />;
  }
  if (mime === "application/pdf" || mime.startsWith("text/") || mime.includes("word")) {
    return <FileText {...props} className={className} />;
  }
  return <File {...props} className={className} />;
}
