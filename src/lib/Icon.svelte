<script lang="ts">
  export type IconName =
    | "route"
    | "user"
    | "settings"
    | "arrow"
    | "external"
    | "shield"
    | "chevron"
    | "check"
    | "sun"
    | "moon"
    | "monitor"
    | "palette"
    | "globe"
    | "info"
    | "key"
    | "clock"
    | "bus"
    | "search"
    | "swap"
    | "pin"
    | "help"
    | "refresh"
    | "sparkles";

  let { name, size = 20 }: { name: IconName; size?: number } = $props();

  const paths: Record<IconName, string> = {
    route:
      "M6 7a3 3 0 1 0 0-6 3 3 0 0 0 0 6Zm12 16a3 3 0 1 0 0-6 3 3 0 0 0 0 6ZM9 4h5a4 4 0 0 1 0 8h-4a4 4 0 0 0 0 8h5",
    user: "M20 21v-2a6 6 0 0 0-6-6h-4a6 6 0 0 0-6 6v2M16 5a4 4 0 1 1-8 0 4 4 0 0 1 8 0Z",
    settings: "M4 7h9m4 0h3M4 17h3m4 0h9M13 4v6M7 14v6",
    arrow: "M4 12h16m-6-6 6 6-6 6",
    external:
      "M14 3h7v7m0-7L10 14M10 3H5a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-5",
    shield: "m12 3 8 3v5c0 5-4 8-8 10-4-2-8-5-8-10V6l8-3Zm-4 9 3 3 5-6",
    chevron: "m9 5 7 7-7 7",
    check: "m5 12 4 4L19 6",
    sun: "M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0ZM12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.5 1.5m11 11L19 19M5 19l1.5-1.5m11-11L19 5",
    moon: "M20.9 13A9 9 0 0 1 11 3.1 9 9 0 1 0 20.9 13Z",
    monitor:
      "M4 3h16a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1ZM8 21h8m-4-4v4",
    palette:
      "M12 3a9 9 0 1 0 0 18h1a2.2 2.2 0 0 0 1.4-3.9 1.5 1.5 0 0 1 1-2.6H18a3 3 0 0 0 3-3A8.7 8.7 0 0 0 12 3ZM7 10h.01M10 6.5h.01M15 7h.01M6.5 14h.01",
    globe:
      "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0ZM3 12h18M12 3c5 5 5 13 0 18-5-5-5-13 0-18Z",
    info: "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0ZM12 11v6m0-10h.01",
    key: "M10 14a6 6 0 1 1 4-4l-9 9H2v-3l3-3m10-8h.01",
    clock: "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0ZM12 7v5l3 2",
    bus: "M5 17V6a3 3 0 0 1 3-3h8a3 3 0 0 1 3 3v11a1 1 0 0 1-1 1H6a1 1 0 0 1-1-1Zm0-6h14M7 18v3m10-3v3M8.5 14.5h.01m7 0h.01M5 8H3v3m16-3h2v3",
    search: "M16.5 10.5a6 6 0 1 1-12 0 6 6 0 0 1 12 0Zm-1.75 4.25L20 20",
    swap: "M8 19V5m-4 4 4-4 4 4m4-4v14m-4-4 4 4 4-4",
    pin: "M19 9c0 5-7 11-7 11S5 14 5 9a7 7 0 1 1 14 0Zm-5 0a2 2 0 1 1-4 0 2 2 0 0 1 4 0Z",
    help: "M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0ZM9.5 8a2.5 2.5 0 1 1 4.2 1.8C12 11 12 11 12 13m0 4h.01",
    refresh:
      "M20 7v5h-5M4 17v-5h5M5.7 6.5A8 8 0 0 1 19 7l1 5M4 12l1 5a8 8 0 0 0 13.3.5",
    sparkles:
      "m12 3 2.5 6.5L21 12l-6.5 2.5L12 21l-2.5-6.5L3 12l6.5-2.5L12 3ZM20 2v4m-2-2h4",
  };
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.65"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d={paths[name]} />
</svg>
