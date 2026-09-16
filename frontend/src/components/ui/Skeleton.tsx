import React from "react"

/** Subtle theme-adaptive loading placeholder (shimmer driven by .skeleton in globals.css). */
export const Skeleton: React.FC<{ className?: string }> = ({ className }) => (
  <div aria-hidden className={`skeleton ${className ?? ""}`} />
)
