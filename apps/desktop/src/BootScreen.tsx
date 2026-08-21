/** Splash a pantalla completa hasta el primer snapshot (PTYs listos). */
export function BootScreen({ error }: { error: string | null }) {
  return (
    <div className="boot" role="status" aria-live="polite" aria-busy={!error}>
      <img className="boot-logo" src={`${import.meta.env.BASE_URL}icon.png`} alt="" width={72} height={72} />
      <p className="boot-word">
        <span className="lol">lol</span>
        <span className="mark">term</span>
      </p>
      <p className={`boot-status${error ? " is-err" : ""}`}>
        {error ? `no arrancó · ${error}` : "abriendo workspace…"}
      </p>
      {!error && <div className="boot-bar" aria-hidden />}
    </div>
  );
}
