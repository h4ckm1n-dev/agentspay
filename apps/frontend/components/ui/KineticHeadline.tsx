"use client";

import { motion, useReducedMotion } from "motion/react";
import type { ReactNode } from "react";

const EASE = [0.22, 1, 0.36, 1] as const;

/**
 * Headline whose lines rise into view from behind a mask. Each line sits in an
 * overflow-hidden block; the inner span translates up from 110% to 0.
 */
export function KineticHeadline({
  lines,
  className,
  lineClassName,
}: {
  lines: ReactNode[];
  className?: string;
  lineClassName?: string;
}) {
  const reduceMotion = useReducedMotion();

  return (
    <h1 className={className}>
      {lines.map((line, i) => (
        <span key={i} className="block overflow-hidden pb-[0.08em]">
          <motion.span
            className={`block ${lineClassName ?? ""}`}
            initial={reduceMotion ? false : { y: "110%" }}
            animate={reduceMotion ? {} : { y: 0 }}
            transition={{ duration: 0.7, delay: 0.06 + i * 0.09, ease: EASE }}
          >
            {line}
          </motion.span>
        </span>
      ))}
    </h1>
  );
}
