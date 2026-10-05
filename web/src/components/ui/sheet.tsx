import * as React from "react"
import { Dialog as SheetPrimitive } from "@base-ui/react/dialog"
import { cn } from "cn"

import { Button } from "@/components/ui/button"
import { XIcon, Maximize2, PanelRightOpen } from "lucide-react"

function Sheet({ ...props }: SheetPrimitive.Root.Props) {
  return <SheetPrimitive.Root data-slot="sheet" {...props} />
}

function SheetTrigger({ ...props }: SheetPrimitive.Trigger.Props) {
  return <SheetPrimitive.Trigger data-slot="sheet-trigger" {...props} />
}

function SheetClose({ ...props }: SheetPrimitive.Close.Props) {
  return <SheetPrimitive.Close data-slot="sheet-close" {...props} />
}

function SheetPortal({ ...props }: SheetPrimitive.Portal.Props) {
  return <SheetPrimitive.Portal data-slot="sheet-portal" {...props} />
}

function SheetOverlay({ className, ...props }: SheetPrimitive.Backdrop.Props) {
  return (
    <SheetPrimitive.Backdrop
      data-slot="sheet-overlay"
      className={cn(
        "fixed inset-0 z-50 bg-black/10 transition-opacity duration-200 data-ending-style:opacity-0 data-starting-style:opacity-0 supports-backdrop-filter:backdrop-blur-xs",
        className
      )}
      {...props}
    />
  )
}

type SheetPosition = "side" | "center";

function SheetContent({
  className,
  children,
  side = "right",
  showCloseButton = true,
  dockable = false,
  ...props
}: SheetPrimitive.Popup.Props & {
  side?: "top" | "right" | "bottom" | "left"
  showCloseButton?: boolean
  dockable?: boolean
}) {
  const [position, setPosition] = React.useState<SheetPosition>("side");

  const isSide = position === "side";
  const isCenter = position === "center";

  return (
    <SheetPortal>
      <SheetOverlay />
      <SheetPrimitive.Popup
        data-slot="sheet-content"
        data-side={side}
        data-position={position}
        className={cn(
          "fixed z-50 flex flex-col gap-4 overflow-x-hidden bg-popover bg-clip-padding text-sm text-popover-foreground shadow-lg outline-none",
          "transition-all duration-300 ease-[cubic-bezier(0.32,0.72,0,1)]",
          "data-ending-style:opacity-0 data-starting-style:opacity-0",
          isSide && side === "right" && [
            "inset-3 h-auto w-auto rounded-xl border",
            "sm:inset-auto sm:top-3 sm:right-3 sm:bottom-3 sm:left-auto sm:w-[min(75%-1.5rem,28rem)]",
            "data-ending-style:translate-y-[2.5rem] sm:data-ending-style:translate-y-0 sm:data-ending-style:translate-x-[2.5rem]",
            "data-starting-style:translate-y-[2.5rem] sm:data-starting-style:translate-y-0 sm:data-starting-style:translate-x-[2.5rem]",
          ],
          isSide && side === "left" && [
            "inset-3 h-auto w-auto rounded-xl border",
            "sm:inset-auto sm:top-3 sm:left-3 sm:bottom-3 sm:right-auto sm:w-[min(75%-1.5rem,28rem)]",
            "data-ending-style:translate-y-[2.5rem] sm:data-ending-style:translate-y-0 sm:data-ending-style:translate-x-[-2.5rem]",
            "data-starting-style:translate-y-[2.5rem] sm:data-starting-style:translate-y-0 sm:data-starting-style:translate-x-[-2.5rem]",
          ],
          isSide && side === "top" && [
            "inset-x-0 top-0 h-auto border-b",
            "data-ending-style:translate-y-[-2.5rem] data-starting-style:translate-y-[-2.5rem]",
          ],
          isSide && side === "bottom" && [
            "inset-x-0 bottom-0 h-auto border-t",
            "data-ending-style:translate-y-[2.5rem] data-starting-style:translate-y-[2.5rem]",
          ],
          isCenter && [
            "left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2",
            "h-[min(85vh,900px)] w-[min(90vw,640px)]",
            "rounded-xl border",
            "data-ending-style:scale-95 data-starting-style:scale-95",
          ],
          className
        )}
        {...props}
      >
        {children}
        <div className="absolute top-3 right-3 flex items-center gap-1">
          {dockable && (
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={() => setPosition(isSide ? "center" : "side")}
              title={isSide ? "На весь екран" : "Вбік"}
            >
              {isSide ? <Maximize2 className="h-4 w-4" /> : <PanelRightOpen className="h-4 w-4" />}
            </Button>
          )}
          {showCloseButton && (
            <SheetPrimitive.Close
              data-slot="sheet-close"
              render={
                <Button variant="ghost" size="icon-sm" />
              }
            >
              <XIcon />
              <span className="sr-only">Close</span>
            </SheetPrimitive.Close>
          )}
        </div>
      </SheetPrimitive.Popup>
    </SheetPortal>
  )
}

function SheetHeader({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="sheet-header"
      className={cn("flex flex-col gap-0.5 p-4 pr-20", className)}
      {...props}
    />
  )
}

function SheetFooter({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="sheet-footer"
      className={cn("mt-auto flex flex-col gap-2 p-4", className)}
      {...props}
    />
  )
}

function SheetTitle({ className, ...props }: SheetPrimitive.Title.Props) {
  return (
    <SheetPrimitive.Title
      data-slot="sheet-title"
      className={cn(
        "font-heading text-base font-medium text-foreground",
        className
      )}
      {...props}
    />
  )
}

function SheetDescription({
  className,
  ...props
}: SheetPrimitive.Description.Props) {
  return (
    <SheetPrimitive.Description
      data-slot="sheet-description"
      className={cn("text-sm text-muted-foreground", className)}
      {...props}
    />
  )
}

export {
  Sheet,
  SheetTrigger,
  SheetClose,
  SheetContent,
  SheetHeader,
  SheetFooter,
  SheetTitle,
  SheetDescription,
}
