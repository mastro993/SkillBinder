import { cva, type VariantProps } from "class-variance-authority";
import { cn } from "@/lib/utils";

// The registry ships one shape. Two call sites stand in for fully rounded primitives, a Badge
// pill and the Progress track, and a square-cornered placeholder there reads as a different
// element once the real content lands.
const skeletonVariants = cva("animate-pulse bg-muted", {
  variants: {
    variant: {
      default: "rounded-md",
      pill: "rounded-full",
    },
  },
  defaultVariants: { variant: "default" },
});

function Skeleton({
  className,
  variant,
  ...props
}: React.ComponentProps<"div"> & VariantProps<typeof skeletonVariants>) {
  return (
    <div
      data-slot="skeleton"
      className={cn(skeletonVariants({ variant }), className)}
      {...props}
    />
  );
}

export { Skeleton };
