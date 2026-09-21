import {
  forwardRef,
  type ComponentPropsWithoutRef,
  type ElementRef,
} from "react";
import { ScrollArea as ScrollAreaPrimitive } from "@base-ui/react/scroll-area";
import { cn } from "@/lib/utils";

type ScrollAreaProps = ComponentPropsWithoutRef<
  typeof ScrollAreaPrimitive.Root
>;

export const ScrollArea = forwardRef<
  ElementRef<typeof ScrollAreaPrimitive.Root>,
  ScrollAreaProps
>(({ className, children, ...props }, ref) => (
  <ScrollAreaPrimitive.Root
    ref={ref}
    className={cn("scroll-area", className)}
    {...props}
  >
    <ScrollAreaPrimitive.Viewport className="scroll-area-viewport">
      <ScrollAreaPrimitive.Content className="scroll-area-content">
        {children}
      </ScrollAreaPrimitive.Content>
    </ScrollAreaPrimitive.Viewport>
    <ScrollBar />
    <ScrollBar orientation="horizontal" />
    <ScrollAreaPrimitive.Corner className="scroll-area-corner" />
  </ScrollAreaPrimitive.Root>
));
ScrollArea.displayName = "ScrollArea";

type ScrollBarProps = ComponentPropsWithoutRef<
  typeof ScrollAreaPrimitive.Scrollbar
>;

export const ScrollBar = forwardRef<
  ElementRef<typeof ScrollAreaPrimitive.Scrollbar>,
  ScrollBarProps
>(({ className, orientation = "vertical", ...props }, ref) => (
  <ScrollAreaPrimitive.Scrollbar
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
    <ScrollAreaPrimitive.Thumb className="scroll-area-thumb" />
  </ScrollAreaPrimitive.Scrollbar>
));
ScrollBar.displayName = "ScrollBar";
