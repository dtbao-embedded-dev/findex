/** A short confirmation or error, floating above the status bar. */
export function Toast({ text, tone }: { text: string; tone: "info" | "error" }) {
  return (
    <div className={tone === "error" ? "toast is-error" : "toast"} role="status">
      {text}
    </div>
  );
}
