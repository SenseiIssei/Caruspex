// The note tab.
import { useEffect, useRef, useState } from "react";
import type { GarageCar } from "../../api";
import { translator } from "../../i18n";
import { setNote } from "../../state/garage";

const tr = translator("caruspex::GaragePage");

// The note is saved shortly after typing stops, and right away when another
// car is picked, so nothing typed gets lost.
export function NoteTab({ car }: { car: GarageCar }) {
  const [text, setText] = useState(car.note);
  const pending = useRef<{ vin: string; text: string } | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const flush = () => {
    if (timer.current) clearTimeout(timer.current);
    timer.current = null;
    const p = pending.current;
    pending.current = null;
    if (p) setNote(p.vin, p.text);
  };

  useEffect(() => {
    flush();
    setText(car.note);
    // Only a change of car resets the field; the note itself follows below.
  }, [car.vin]);

  useEffect(() => {
    // A change from elsewhere (an import) shows, unless something is being typed.
    if (!pending.current && car.note !== text) setText(car.note);
  }, [car.note]);

  useEffect(() => () => flush(), []);

  return (
    <div className="garage-tab">
      <textarea
        className="textarea garage-note"
        value={text}
        placeholder={tr("z.B. Zahnriemen bei 180 000 km gewechselt")}
        onChange={(e) => {
          setText(e.target.value);
          pending.current = { vin: car.vin, text: e.target.value };
          if (timer.current) clearTimeout(timer.current);
          timer.current = setTimeout(flush, 700);
        }}
      />
    </div>
  );
}
