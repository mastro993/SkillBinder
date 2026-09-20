import {
  forwardRef,
  type ComponentPropsWithoutRef,
  type ElementRef,
} from "react";
import * as ScrollAreaPrimitive from "@radix-ui/react-scroll-area";
import { cn } from "./utils";

type ScrollAreaProps = ComponentPropsWithoutRef<
  typeof ScrollAreaPrimitive.Root
>;

export const ScrollArea = forwardRef<
  ElementRef<typeof ScrollAreaPrimitive.Root>,
  ScrollAreaProps
>(({ className, children, type = "auto", ...props }, ref) => (
  <ScrollAreaPrimitive.Root
    ref={ref}
    type={type}
    className={cn("scroll-area", className)}
    {...props}
  >
    <ScrollAreaPrimitive.Viewport className="scroll-area-viewport">
      {children}
    </ScrollAreaPrimitive.Viewport>
    <ScrollBar />
    <ScrollBar orientation="horizontal" />
    <ScrollAreaPrimitive.Corner className="scroll-area-corner" />
  </ScrollAreaPrimitive.Root>
));
ScrollArea.displayName = "ScrollArea";

type ScrollBarProps = ComponentPropsWithoutRef<
  typeof ScrollAreaPrimitive.ScrollAreaScrollbar
>;

export const ScrollBar = forwardRef<
  ElementRef<typeof ScrollAreaPrimitive.ScrollAreaScrollbar>,
  ScrollBarProps
>(({ className, orientation = "vertical", ...props }, ref) => (
  <ScrollAreaPrimitive.ScrollAreaScrollbar
    ref={ref}
    orientation={orientation}
    className={cn(
      "scroll-area-scrollbar",
      orientation === "vertical"
        ? "scroll-area-vertical"
        : "scroll-area-horizontal",
      className,
    )}
    {...props}
  >
    <ScrollAreaPrimitive.ScrollAreaThumb className="scroll-area-thumb" />
  </ScrollAreaPrimitive.ScrollAreaScrollbar>
));
ScrollBar.displayName = "ScrollBar";
