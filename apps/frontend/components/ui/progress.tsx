import { cn } from "@/lib/utils";

interface ProgressProps {
  value: number;
  className?: string;
  tone?: "default" | "success" | "warning" | "danger";
}

const toneClasses: Record<NonNullable<ProgressProps["tone"]>, string> = {
  default: "bg-primary",
  success: "bg-emerald-500",
  warning: "bg-amber-500",
  danger: "bg-red-500"
};

export function Progress({ value, className, tone = "default" }: ProgressProps) {
  const boundedValue = Math.min(Math.max(value, 0), 100);

  return (
    <div className={cn("h-2 overflow-hidden rounded-full bg-muted", className)}>
      <div
        className={cn("h-full rounded-full transition-all", toneClasses[tone])}
        style={{ width: `${boundedValue}%` }}
      />
    </div>
  );
}
