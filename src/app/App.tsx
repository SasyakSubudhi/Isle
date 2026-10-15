export function App() {
  return (
    <main className="island-canvas select-none">
      <div className="island-pill" role="img" aria-label="Isle is ready">
        <svg width="19" height="19" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path d="M4 15c2-5 4-5 6 0s4 5 6 0 3-5 4-2" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" />
          <path d="M7 8h10" stroke="currentColor" strokeOpacity=".45" strokeWidth="1.5" strokeLinecap="round" />
        </svg>
        <span>isle</span>
        <span className="ready-dot" />
      </div>
    </main>
  );
}
