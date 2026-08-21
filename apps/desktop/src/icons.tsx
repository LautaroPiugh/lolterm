import type { CSSProperties, ReactNode } from "react";

type IconProps = {
  size?: number;
  color?: string;
  style?: CSSProperties;
  strokeWidth?: number;
  className?: string;
};

function Icon({
  size = 16,
  color = "currentColor",
  style,
  strokeWidth = 1.75,
  className,
  children,
}: IconProps & { children: ReactNode }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke={color}
      strokeWidth={strokeWidth}
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      style={{ flexShrink: 0, ...style }}
    >
      {children}
    </svg>
  );
}

export function Home(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
      <polyline points="9 22 9 12 15 12 15 22" />
    </Icon>
  );
}

export function Files(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" />
      <path d="M14 2v4a2 2 0 0 0 2 2h4" />
      <path d="M10 9H8" />
      <path d="M16 13H8" />
      <path d="M16 17H8" />
    </Icon>
  );
}

export function GitBranch(p: IconProps) {
  return (
    <Icon {...p}>
      <line x1="6" y1="3" x2="6" y2="15" />
      <circle cx="18" cy="6" r="3" />
      <circle cx="6" cy="18" r="3" />
      <path d="M18 9a9 9 0 0 1-9 9" />
    </Icon>
  );
}

export function Terminal(p: IconProps) {
  return (
    <Icon {...p}>
      <polyline points="4 17 10 11 4 5" />
      <line x1="12" y1="19" x2="20" y2="19" />
    </Icon>
  );
}

export function Cloud(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M18 10h-1.26A8 8 0 1 0 9 20h9a5 5 0 0 0 0-10z" />
    </Icon>
  );
}

export function ChevronRight(p: IconProps) {
  return (
    <Icon {...p}>
      <polyline points="9 18 15 12 9 6" />
    </Icon>
  );
}

export function ChevronDown(p: IconProps) {
  return (
    <Icon {...p}>
      <polyline points="6 9 12 15 18 9" />
    </Icon>
  );
}

export function Minus(p: IconProps) {
  return (
    <Icon {...p}>
      <line x1="5" y1="12" x2="19" y2="12" />
    </Icon>
  );
}

export function Square(p: IconProps) {
  return (
    <Icon {...p}>
      <rect x="3" y="3" width="18" height="18" rx="2" ry="2" />
    </Icon>
  );
}

export function X(p: IconProps) {
  return (
    <Icon {...p}>
      <line x1="18" y1="6" x2="6" y2="18" />
      <line x1="6" y1="6" x2="18" y2="18" />
    </Icon>
  );
}

export function Plus(p: IconProps) {
  return (
    <Icon {...p}>
      <line x1="12" y1="5" x2="12" y2="19" />
      <line x1="5" y1="12" x2="19" y2="12" />
    </Icon>
  );
}

export function Folder(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
    </Icon>
  );
}

export function FolderOpen(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
      <polyline points="2 12 6 8 22 8" />
    </Icon>
  );
}

export function FileCode(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
      <polyline points="14 2 14 8 20 8" />
      <polyline points="10 13 8 15 10 17" />
      <polyline points="14 13 16 15 14 17" />
    </Icon>
  );
}

export function FileText(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
      <polyline points="14 2 14 8 20 8" />
      <line x1="16" y1="13" x2="8" y2="13" />
      <line x1="16" y1="17" x2="8" y2="17" />
    </Icon>
  );
}

export function Sparkles(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M12 3l1.5 4.5L18 9l-4.5 1.5L12 15l-1.5-4.5L6 9l4.5-1.5z" />
      <path d="M5 17l.8 2.2L8 20l-2.2.8L5 23l-.8-2.2L2 20l2.2-.8z" />
      <path d="M19 3l.6 1.8L21 5.4l-1.4.6L19 8l-.6-1.6L17 5.4l1.4-.6z" />
    </Icon>
  );
}

export function Network(p: IconProps) {
  return (
    <Icon {...p}>
      <rect x="9" y="2" width="6" height="4" />
      <rect x="16" y="10" width="6" height="4" />
      <rect x="2" y="10" width="6" height="4" />
      <rect x="9" y="18" width="6" height="4" />
      <path d="M12 6v4" />
      <path d="M12 14v4" />
      <path d="M8 12H5" />
      <path d="M19 12h-3" />
    </Icon>
  );
}

export function Server(p: IconProps) {
  return (
    <Icon {...p}>
      <rect x="2" y="2" width="20" height="8" rx="2" />
      <rect x="2" y="14" width="20" height="8" rx="2" />
      <line x1="6" y1="6" x2="6.01" y2="6" />
      <line x1="6" y1="18" x2="6.01" y2="18" />
    </Icon>
  );
}

export function FolderPlus(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
      <line x1="12" y1="11" x2="12" y2="17" />
      <line x1="9" y1="14" x2="15" y2="14" />
    </Icon>
  );
}

export function Check(p: IconProps) {
  return (
    <Icon {...p}>
      <polyline points="20 6 9 17 4 12" />
    </Icon>
  );
}

export function Command(p: IconProps) {
  return (
    <Icon {...p}>
      <path d="M18 3a3 3 0 0 0-3 3v12a3 3 0 0 0 3 3 3 3 0 0 0 3-3 3 3 0 0 0-3-3H6a3 3 0 0 0-3 3 3 3 0 0 0 3 3 3 3 0 0 0 3-3V6a3 3 0 0 0-3-3 3 3 0 0 0-3 3 3 3 0 0 0 3 3h12a3 3 0 0 0 3-3 3 3 0 0 0-3-3z" />
    </Icon>
  );
}

export function GitCommitHorizontal(p: IconProps) {
  return (
    <Icon {...p}>
      <circle cx="12" cy="12" r="3" />
      <line x1="3" y1="12" x2="9" y2="12" />
      <line x1="15" y1="12" x2="21" y2="12" />
    </Icon>
  );
}

export function Columns(p: IconProps) {
  return (
    <Icon {...p}>
      <rect x="3" y="3" width="18" height="18" rx="2" />
      <line x1="12" y1="3" x2="12" y2="21" />
    </Icon>
  );
}

export function Rows(p: IconProps) {
  return (
    <Icon {...p}>
      <rect x="3" y="3" width="18" height="18" rx="2" />
      <line x1="3" y1="12" x2="21" y2="12" />
    </Icon>
  );
}

export function Copy(p: IconProps) {
  return (
    <Icon {...p}>
      <rect x="9" y="9" width="13" height="13" rx="2" />
      <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
    </Icon>
  );
}

export function Pause(p: IconProps) {
  return (
    <Icon {...p}>
      <rect x="6" y="4" width="4" height="16" />
      <rect x="14" y="4" width="4" height="16" />
    </Icon>
  );
}

export function Play(p: IconProps) {
  return (
    <Icon {...p}>
      <polygon points="6 4 20 12 6 20 6 4" />
    </Icon>
  );
}

export function SkipBack(p: IconProps) {
  return (
    <Icon {...p}>
      <polygon points="19 20 9 12 19 4 19 20" />
      <line x1="5" y1="19" x2="5" y2="5" />
    </Icon>
  );
}

export function SkipForward(p: IconProps) {
  return (
    <Icon {...p}>
      <polygon points="5 4 15 12 5 20 5 4" />
      <line x1="19" y1="5" x2="19" y2="19" />
    </Icon>
  );
}

export function Volume2(p: IconProps) {
  return (
    <Icon {...p}>
      <polygon points="11 5 6 9 2 9 2 15 6 15 11 19 11 5" />
      <path d="M15.54 8.46a5 5 0 0 1 0 7.07" />
      <path d="M19.07 4.93a10 10 0 0 1 0 14.14" />
    </Icon>
  );
}

export function RotateCcw(p: IconProps) {
  return (
    <Icon {...p}>
      <polyline points="1 4 1 10 7 10" />
      <path d="M3.51 15a9 9 0 1 0 2.13-9.36L1 10" />
    </Icon>
  );
}

export function Settings(p: IconProps) {
  return (
    <Icon {...p}>
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
    </Icon>
  );
}

