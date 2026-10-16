import type { SVGProps } from 'react';
const paths = {
  shelf: 'M4 6h16v12H4z M4 13h4l2 3h4l2-3h4',
  clipboard: 'M9 4h6v3H9z M9 5H6v15h12V5h-3',
  usage: 'M5 19V9m7 10V4m7 15v-7',
  dictionary: 'M4 5h6c2 0 2 2 2 2s0-2 2-2h6v14h-6c-2 0-2 2-2 2s0-2-2-2H4z M12 7v14',
  focus: 'M12 7v5l3 2 M9 2h6 M12 2v3 M19 5l2 2 M20 13a8 8 0 1 1-16 0 8 8 0 0 1 16 0',
  media:
    'M9 17V5l10-2v12 M9 8l10-2 M9 17a3 2 0 1 1-3-2c1 0 3 0 3 2 M19 15a3 2 0 1 1-3-2c1 0 3 0 3 2',
  notes: 'M5 3h14v18H5z M8 7h8 M8 11h8 M8 15h5',
  settings:
    'M9 4l1-2h4l1 2 3 2 2 1v4l-2 1-1 3-1 2h-4l-1-2-3-2-2-1v-4l2-1z M15 9a3 3 0 1 1-6 0 3 3 0 0 1 6 0',
  pin: 'M8 3h8l-1 6 3 3H6l3-3z M12 12v9',
  close: 'M6 6l12 12 M18 6L6 18',
  plus: 'M12 5v14 M5 12h14',
  copy: 'M8 8h12v12H8z M4 16V4h12',
  more: 'M5 12h.01 M12 12h.01 M19 12h.01',
  trash: 'M3 6h18 M9 6V3h6v3 M6 6l1 15h10l1-15 M10 10v7 M14 10v7',
  file: 'M6 2h8l4 4v16H6z M14 2v5h4 M9 12h6 M9 16h6',
  link: 'M9 15l6-6 M8 11L6 13a3.5 3.5 0 0 0 5 5l2-2 M13 8l2-2a3.5 3.5 0 0 1 5 5l-2 2',
  image: 'M3 3h18v18H3z M3 17l5-5 4 4 3-3 6 6 M8 7h.01',
  folder: 'M3 6h7l2 2h9v12H3z',
  arrow: 'M15 5l-7 7 7 7',
  check: 'M5 12l4 4L19 6',
  refresh: 'M20 7v5h-5 M4 17v-5h5 M19 8a8 8 0 0 0-13-3 M5 16a8 8 0 0 0 13 3',
  capture: 'M4 8V4h4 M16 4h4v4 M20 16v4h-4 M8 20H4v-4 M8 8h8v8H8z',
  privacy:
    'M3 3l18 18 M10 5a11 11 0 0 1 11 7l-3 4 M14 19a11 11 0 0 1-11-7l3-4 M10 10a3 3 0 0 0 4 4',
  play: 'M8 4l12 8-12 8z',
  pause: 'M8 5v14 M16 5v14',
  next: 'M5 5l10 7-10 7z M19 5v14',
  previous: 'M19 5L9 12l10 7z M5 5v14',
  volume: 'M3 9h4l5-4v14l-5-4H3z M16 8a6 6 0 0 1 0 8',
  mail: 'M3 5h18v14H3z M3 5l9 8 9-8',
  chat: 'M4 4h16v13H9l-5 4z M8 8h8 M8 12h5',
  chrome:
    'M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0 M15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0 M12 3l5 9 M4 7h10 M7 20l5-8',
  spark: 'M12 3l2 6 7 3-7 2-2 7-2-7-7-2 7-3z',
  expand: 'M9 3H3v6 M15 3h6v6 M21 15v6h-6 M3 15v6h6',
  save: 'M5 3h12l3 3v15H4V3z M8 3v6h8V3 M8 21v-7h8v7',
  power: 'M12 3v8 M6.3 6.8a8 8 0 1 0 11.4 0',
  widgets: 'M4 4h7v7H4z M13 4h7v7h-7z M4 13h7v7H4z M13 13h7v7h-7z',
} satisfies Record<string, string>;
export type IconName = keyof typeof paths;
export function Icon({
  name,
  size = 18,
  ...props
}: SVGProps<SVGSVGElement> & { name: IconName; size?: number }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
      {...props}
    >
      <path d={paths[name]} />
    </svg>
  );
}
