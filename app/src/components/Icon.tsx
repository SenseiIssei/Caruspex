// The line icons of the Qt version: 24 x 24, drawn with the current text
// colour, round caps and joins. Crisp at every size because they stay SVG.
import type { CSSProperties } from "react";

const SHAPES = {
  "home": "<path d=\"M3 10.5 12 3l9 7.5\"/><path d=\"M5 9.5V20h5v-6h4v6h5V9.5\"/>",
  "alert": "<path d=\"M12 3.5 2.5 20h19z\"/><path d=\"M12 10v4.5\"/><circle cx=\"12\" cy=\"17.3\" r=\".6\" fill=\"currentColor\"/>",
  "activity": "<path d=\"M2.5 12h4l2.5-7 5 14 2.5-7h5\"/>",
  "wrench": "<path d=\"M14.5 6.5a4 4 0 0 0 5 5L21 13l-8 8-3-3 8-8-1.5-1.5a4 4 0 0 0-5-5l2.5 2.5-1 2-2 1z\"/><path d=\"M3 21l7-7\"/>",
  "gauge": "<path d=\"M4 18a9 9 0 1 1 16 0\"/><path d=\"M12 13l4-5\"/><circle cx=\"12\" cy=\"13\" r=\"1.2\"/>",
  "throttle": "<circle cx=\"12\" cy=\"12\" r=\"8.5\"/><path d=\"M6.5 17.5 17.5 6.5\"/><circle cx=\"12\" cy=\"12\" r=\"1.3\"/>",
  "chart": "<path d=\"M4 20V4\"/><path d=\"M4 20h16\"/><path d=\"M8 16v-5\"/><path d=\"M12 16V8\"/><path d=\"M16 16v-3\"/><path d=\"M20 16V6\"/>",
  "cpu": "<rect x=\"6\" y=\"6\" width=\"12\" height=\"12\" rx=\"2\"/><rect x=\"9.5\" y=\"9.5\" width=\"5\" height=\"5\" rx=\"1\"/><path d=\"M9 3v3M15 3v3M9 18v3M15 18v3M3 9h3M3 15h3M18 9h3M18 15h3\"/>",
  "sliders": "<path d=\"M4 6h10M18 6h2M4 12h4M12 12h8M4 18h12M20 18h0\"/><circle cx=\"16\" cy=\"6\" r=\"2\"/><circle cx=\"10\" cy=\"12\" r=\"2\"/><circle cx=\"18\" cy=\"18\" r=\"2\"/>",
  "book": "<path d=\"M4 4.5A1.5 1.5 0 0 1 5.5 3H20v15H5.5A1.5 1.5 0 0 0 4 19.5z\"/><path d=\"M4 19.5A1.5 1.5 0 0 0 5.5 21H20v-3\"/><path d=\"M8 7h8M8 10.5h6\"/>",
  "settings": "<circle cx=\"12\" cy=\"12\" r=\"3\"/><path d=\"M12 2.5v3M12 18.5v3M2.5 12h3M18.5 12h3M5.3 5.3l2.1 2.1M16.6 16.6l2.1 2.1M5.3 18.7l2.1-2.1M16.6 7.4l2.1-2.1\"/>",
  "car": "<path d=\"M3 15.5V12l2.2-4.4A2 2 0 0 1 7 6.5h10a2 2 0 0 1 1.8 1.1L21 12v3.5a1 1 0 0 1-1 1h-1\"/><path d=\"M5 16.5H4a1 1 0 0 1-1-1\"/><path d=\"M9 16.5h6\"/><circle cx=\"7\" cy=\"16.5\" r=\"2\"/><circle cx=\"17\" cy=\"16.5\" r=\"2\"/><path d=\"M4 12h16\"/>",
  "battery": "<rect x=\"2.5\" y=\"7\" width=\"17\" height=\"10\" rx=\"2\"/><path d=\"M21.5 10.5v3\"/><path d=\"M6.5 12h3M8 10.5v3M13 12h3\"/>",
  "route": "<circle cx=\"6\" cy=\"18\" r=\"2.2\"/><circle cx=\"18\" cy=\"6\" r=\"2.2\"/><path d=\"M8.2 18H15a3 3 0 0 0 0-6H9a3 3 0 0 1 0-6h6.8\"/>",
  "timer": "<circle cx=\"12\" cy=\"13.5\" r=\"7.5\"/><path d=\"M12 13.5V9.5\"/><path d=\"M9.5 2.5h5\"/><path d=\"M18.5 6.5l1.5-1.5\"/>",
  "heart": "<path d=\"M12 20s-7.5-4.6-7.5-10A4.3 4.3 0 0 1 12 7.2 4.3 4.3 0 0 1 19.5 10c0 5.4-7.5 10-7.5 10z\"/><path d=\"M7 12.5h2.5l1.2-2 2 4 1.3-2H17\"/>",
  "garage": "<path d=\"M3 20V9l9-5.5L21 9v11\"/><path d=\"M6.5 20v-8h11v8\"/><path d=\"M6.5 15h11M6.5 17.5h11\"/>",
  "file": "<path d=\"M14 3H6.5A1.5 1.5 0 0 0 5 4.5v15A1.5 1.5 0 0 0 6.5 21h11a1.5 1.5 0 0 0 1.5-1.5V8z\"/><path d=\"M14 3v5h5\"/><path d=\"M8.5 13h7M8.5 16.5h5\"/>",
  "plug": "<path d=\"M9 2.5v5M15 2.5v5\"/><path d=\"M6.5 7.5h11V11a5.5 5.5 0 0 1-11 0z\"/><path d=\"M12 16.5v5\"/>",
  "unplug": "<path d=\"M9 2.5v5M15 2.5v5\"/><path d=\"M6.5 7.5h11V11a5.5 5.5 0 0 1-11 0z\"/><path d=\"M12 16.5v5\"/><path d=\"M3 3l18 18\"/>",
  "wifi": "<path d=\"M2.5 9a14 14 0 0 1 19 0\"/><path d=\"M5.5 12.5a9.5 9.5 0 0 1 13 0\"/><path d=\"M8.7 16a5 5 0 0 1 6.6 0\"/><circle cx=\"12\" cy=\"19.3\" r=\".7\" fill=\"currentColor\"/>",
  "bluetooth": "<path d=\"M7 7.5l10 9-5 4.5V3l5 4.5-10 9\"/>",
  "ethernet": "<rect x=\"3.5\" y=\"6\" width=\"17\" height=\"12\" rx=\"2\"/><path d=\"M8 18v-4h8v4\"/><path d=\"M9.5 14v-2M12 14v-2M14.5 14v-2\"/>",
  "usb": "<path d=\"M12 21V5\"/><path d=\"M9.5 6 12 2.5 14.5 6\"/><path d=\"M12 15l-5-3V9\"/><path d=\"M12 17l5-3v-2.5\"/><circle cx=\"7\" cy=\"8.2\" r=\"1.2\"/><rect x=\"15.8\" y=\"9.3\" width=\"2.4\" height=\"2.2\"/><circle cx=\"12\" cy=\"20\" r=\"1.5\"/>",
  "check": "<path d=\"M4.5 12.5 9.5 17.5 19.5 6.5\"/>",
  "check-circle": "<circle cx=\"12\" cy=\"12\" r=\"9\"/><path d=\"M8 12.3l2.8 2.8L16.2 9.5\"/>",
  "x": "<path d=\"M6 6l12 12M18 6 6 18\"/>",
  "x-circle": "<circle cx=\"12\" cy=\"12\" r=\"9\"/><path d=\"M9 9l6 6M15 9l-6 6\"/>",
  "chevron-right": "<path d=\"M9.5 6l6 6-6 6\"/>",
  "chevron-down": "<path d=\"M6 9.5l6 6 6-6\"/>",
  "chevron-left": "<path d=\"M14.5 6l-6 6 6 6\"/>",
  "info": "<circle cx=\"12\" cy=\"12\" r=\"9\"/><path d=\"M12 11v5.5\"/><circle cx=\"12\" cy=\"7.8\" r=\".6\" fill=\"currentColor\"/>",
  "search": "<circle cx=\"11\" cy=\"11\" r=\"6.5\"/><path d=\"M16 16l4.5 4.5\"/>",
  "play": "<path d=\"M7 4.5v15l12-7.5z\"/>",
  "stop": "<rect x=\"6\" y=\"6\" width=\"12\" height=\"12\" rx=\"2\"/>",
  "pause": "<path d=\"M8.5 5v14M15.5 5v14\"/>",
  "download": "<path d=\"M12 3.5v11\"/><path d=\"M7.5 10.5 12 15l4.5-4.5\"/><path d=\"M4 16.5V19a1.5 1.5 0 0 0 1.5 1.5h13A1.5 1.5 0 0 0 20 19v-2.5\"/>",
  "trash": "<path d=\"M4 6.5h16\"/><path d=\"M9 6.5V4h6v2.5\"/><path d=\"M6 6.5l1 13A1.5 1.5 0 0 0 8.5 21h7a1.5 1.5 0 0 0 1.5-1.5l1-13\"/><path d=\"M10 11v6M14 11v6\"/>",
  "refresh": "<path d=\"M20 11.5A8 8 0 0 0 6 6.5L4 8.5\"/><path d=\"M4 4v4.5h4.5\"/><path d=\"M4 12.5a8 8 0 0 0 14 5l2-2\"/><path d=\"M20 20v-4.5h-4.5\"/>",
  "sparkles": "<path d=\"M10 3.5l1.6 4.4L16 9.5l-4.4 1.6L10 15.5l-1.6-4.4L4 9.5l4.4-1.6z\"/><path d=\"M17.5 14l.8 2.2 2.2.8-2.2.8-.8 2.2-.8-2.2-2.2-.8 2.2-.8z\"/>",
  "shield": "<path d=\"M12 3l7.5 3v5.5c0 4.6-3.2 8.2-7.5 9.5-4.3-1.3-7.5-4.9-7.5-9.5V6z\"/><path d=\"M8.5 12l2.5 2.5 4.5-5\"/>",
  "terminal": "<rect x=\"3\" y=\"4.5\" width=\"18\" height=\"15\" rx=\"2\"/><path d=\"M7 9.5l3 2.5-3 2.5\"/><path d=\"M12.5 15h4.5\"/>",
  "minimise": "<path d=\"M6 12h12\"/>",
  "maximise": "<rect x=\"6\" y=\"6\" width=\"12\" height=\"12\" rx=\"1.5\"/>",
  "restore": "<rect x=\"5\" y=\"8\" width=\"11\" height=\"11\" rx=\"1.5\"/><path d=\"M8.5 5H17.5A1.5 1.5 0 0 1 19 6.5V15.5\"/>",
  "stethoscope": "<path d=\"M6 3.5v5a4 4 0 0 0 8 0v-5\"/><path d=\"M10 12.5v2a5 5 0 0 0 10 0v-2\"/><circle cx=\"20\" cy=\"10.5\" r=\"2\"/>",
  "leaf": "<path d=\"M5 19c0-8 5-13.5 15-14-0.5 10-6 15-14 15\"/><path d=\"M5 19c3-4 6-6.5 9-8\"/>",
  "fuel": "<path d=\"M4 21V5.5A1.5 1.5 0 0 1 5.5 4h7A1.5 1.5 0 0 1 14 5.5V21\"/><path d=\"M3 21h12\"/><path d=\"M6.5 8h5\"/><path d=\"M14 11h2a1.5 1.5 0 0 1 1.5 1.5V16a1.5 1.5 0 0 0 3 0V8.5L18 6\"/>",
  "thermometer": "<path d=\"M14 14.8V5a2 2 0 0 0-4 0v9.8a4 4 0 1 0 4 0z\"/><path d=\"M12 9v7\"/>",
  "zap": "<path d=\"M13 2.5 4.5 13.5H12l-1 8 8.5-11H12z\"/>",
  "list": "<path d=\"M9 6.5h11M9 12h11M9 17.5h11\"/><circle cx=\"4.5\" cy=\"6.5\" r=\".9\" fill=\"currentColor\"/><circle cx=\"4.5\" cy=\"12\" r=\".9\" fill=\"currentColor\"/><circle cx=\"4.5\" cy=\"17.5\" r=\".9\" fill=\"currentColor\"/>",
  "engine": "<path d=\"M4 9.5h2V7h4.5V5.5h4V7H17l2 2.5h2v6h-2L17 18H8l-2-2.5H4z\"/><path d=\"M2 11v3\"/><path d=\"M11 10l-1.5 3h3L11 16\"/>",
  "cable": "<path d=\"M4.5 9V5.5\"/><path d=\"M7.5 9V5.5\"/><rect x=\"3\" y=\"9\" width=\"6\" height=\"4\" rx=\"1\"/><path d=\"M6 13v2a4 4 0 0 0 4 4h4a4 4 0 0 0 4-4V9\"/><rect x=\"15\" y=\"5\" width=\"6\" height=\"4\" rx=\"1\"/>",
  "menu": "<path d=\"M4 6.5h16M4 12h16M4 17.5h16\"/>",
  "sidebar": "<rect x=\"3\" y=\"4\" width=\"18\" height=\"16\" rx=\"2\"/><path d=\"M9 4v16\"/>",
  "clock": "<circle cx=\"12\" cy=\"12\" r=\"9\"/><path d=\"M12 7v5l3.5 2\"/>",
  "key": "<circle cx=\"8\" cy=\"15\" r=\"4\"/><path d=\"M11 12l9-9\"/><path d=\"M17 6l2.5 2.5\"/><path d=\"M14.5 8.5l2 2\"/>",
  "cube": "<path d=\"M12 2.8 20.5 7.5v9L12 21.2 3.5 16.5v-9z\"/><path d=\"M3.5 7.5 12 12.2l8.5-4.7\"/><path d=\"M12 12.2v9\"/>",
  "map-pin": "<path d=\"M12 21s-6.5-6-6.5-11a6.5 6.5 0 0 1 13 0c0 5-6.5 11-6.5 11z\"/><circle cx=\"12\" cy=\"10\" r=\"2.3\"/>",
  "lightbulb": "<path d=\"M9 18h6M10 21h4\"/><path d=\"M12 3a6 6 0 0 0-3.6 10.8c.7.6 1.1 1.3 1.1 2.2h5c0-.9.4-1.6 1.1-2.2A6 6 0 0 0 12 3z\"/>",
  "printer": "<path d=\"M7 8V3.5h10V8\"/><rect x=\"3\" y=\"8\" width=\"18\" height=\"9\" rx=\"2\"/><path d=\"M7 14h10v6.5H7z\"/>",
  "steering": "<circle cx=\"12\" cy=\"12\" r=\"9\"/><circle cx=\"12\" cy=\"12\" r=\"2.5\"/><path d=\"M3.5 10.5h6M14.5 10.5h6M12 14.5V21\"/>",
  "bolt-circle": "<circle cx=\"12\" cy=\"12\" r=\"9\"/><path d=\"M13 6.5 8.5 13H12l-1 4.5 4.5-6.5H12z\"/>",
} as const;

export type IconName = keyof typeof SHAPES;

export const ICON_NAMES = Object.keys(SHAPES) as IconName[];

export function Icon({
  name,
  size = 18,
  stroke = 1.8,
  className,
  style,
  title,
}: {
  name: IconName;
  size?: number;
  stroke?: number;
  className?: string;
  style?: CSSProperties;
  title?: string;
}) {
  return (
    <svg
      className={className ? `icon ${className}` : "icon"}
      style={style}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={stroke}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden={title ? undefined : true}
      role={title ? "img" : undefined}
      dangerouslySetInnerHTML={{ __html: (title ? `<title>${title}</title>` : "") + SHAPES[name] }}
    />
  );
}
