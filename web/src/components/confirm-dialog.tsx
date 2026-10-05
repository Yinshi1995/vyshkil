import { useState, useCallback, useRef } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

interface ConfirmOptions {
  title: string;
  description?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  variant?: "default" | "destructive";
  input?: { label: string; placeholder?: string; type?: string };
}

type ResolveRef = ((value: string | boolean) => void) | null;

export function useConfirm() {
  const [state, setState] = useState<(ConfirmOptions & { open: boolean }) | null>(null);
  const resolveRef = useRef<ResolveRef>(null);

  const confirm = useCallback((opts: ConfirmOptions): Promise<boolean> => {
    return new Promise((resolve) => {
      resolveRef.current = resolve as ResolveRef;
      setState({ ...opts, open: true });
    });
  }, []);

  const prompt = useCallback((opts: ConfirmOptions & { input: { label: string; placeholder?: string; type?: string } }): Promise<string | null> => {
    return new Promise((resolve) => {
      resolveRef.current = ((v: string | boolean) => resolve(v === false ? null : v as string)) as ResolveRef;
      setState({ ...opts, open: true });
    });
  }, []);

  function handleClose(result: string | boolean) {
    resolveRef.current?.(result);
    resolveRef.current = null;
    setState(null);
  }

  const dialog = state ? (
    <ConfirmDialogInner
      {...state}
      onClose={handleClose}
    />
  ) : null;

  return { confirm, prompt, dialog };
}

function ConfirmDialogInner({
  open,
  title,
  description,
  confirmLabel = "Підтвердити",
  cancelLabel = "Скасувати",
  variant = "default",
  input,
  onClose,
}: ConfirmOptions & { open: boolean; onClose: (result: string | boolean) => void }) {
  const [inputValue, setInputValue] = useState("");

  return (
    <Dialog open={open} onOpenChange={(o) => { if (!o) onClose(false); }}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          {description && <DialogDescription>{description}</DialogDescription>}
        </DialogHeader>
        {input && (
          <div className="flex flex-col gap-1.5 pt-2">
            <Label>{input.label}</Label>
            <Input
              type={input.type ?? "text"}
              placeholder={input.placeholder}
              value={inputValue}
              onChange={(e) => setInputValue(e.target.value)}
              autoFocus
              onKeyDown={(e) => {
                if (e.key === "Enter" && inputValue.trim()) {
                  onClose(inputValue.trim());
                }
              }}
            />
          </div>
        )}
        <div className="flex justify-end gap-2 pt-2">
          <Button variant="outline" onClick={() => onClose(false)}>
            {cancelLabel}
          </Button>
          <Button
            variant={variant === "destructive" ? "destructive" : "default"}
            onClick={() => onClose(input ? inputValue.trim() || false : true)}
            disabled={!!input && !inputValue.trim()}
          >
            {confirmLabel}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
