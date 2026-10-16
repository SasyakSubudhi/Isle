/** Decorative sky uses only local CSS/SVG; no video, network or canvas polling. */
export function CosmicSky() {
  return (
    <span className="cosmic-sky" aria-hidden="true">
      <span className="nebula nebula-near" /><span className="nebula nebula-far" />
      <svg className="cosmic-stars" viewBox="0 0 148 36" preserveAspectRatio="none">
        <circle cx="54" cy="8" r=".55" /><circle cx="96" cy="27" r=".65" />
        <circle cx="119" cy="10" r=".8" /><circle cx="45" cy="26" r=".4" />
        <circle cx="109" cy="22" r=".4" /><circle cx="79" cy="5" r=".35" />
        <path d="M128 21v4m-2-2h4" stroke="currentColor" strokeWidth=".55" strokeLinecap="round" />
      </svg>
      <span className="orbit-trail" />
      <span className="cosmic-comet" />
    </span>
  );
}
