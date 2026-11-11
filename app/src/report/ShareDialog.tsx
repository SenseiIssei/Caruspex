// "Bericht teilen": the summary as a QR code to scan with a phone, and the
// same text to copy. Nothing leaves the computer; the phone reads the text
// straight from the picture.
import { useMemo } from "react";
import { createPortal } from "react-dom";
import { Modal } from "../components/Modal";
import { toast } from "../components/Toast";
import { Button } from "../components/ui";
import { translator } from "../i18n";
import { encodeQr, qrPath } from "../lib/qr";
import "./ShareDialog.css";

const tu = translator("caruspex::Usability");

export function ShareDialog({ text, onClose }: { text: string; onClose: () => void }) {
  const code = useMemo(() => encodeQr(text, "M"), [text]);
  const quiet = 4;
  const side = code.size + quiet * 2;

  const copy = async () => {
    try {
      await navigator.clipboard.writeText(text);
      toast(tu("Text kopiert"), "success");
    } catch {
      toast(tu("Kopieren ging nicht, bitte den Text markieren."), "warning");
    }
  };

  // Into the body, so it also opens above the report preview.
  return createPortal(
    <Modal
      small
      title={tu("Bericht teilen")}
      icon="qr"
      onClose={onClose}
      footer={
        <>
          <Button onClick={() => void copy()}>{tu("Text kopieren")}</Button>
          <Button variant="primary" onClick={onClose}>
            {tu("Fertig")}
          </Button>
        </>
      }
    >
      <div className="share-body">
        <svg
          className="share-qr"
          viewBox={`0 0 ${side} ${side}`}
          shapeRendering="crispEdges"
          role="img"
          aria-label={tu("QR-Code mit dieser Zusammenfassung")}
        >
          <rect width={side} height={side} fill="#ffffff" />
          <path d={qrPath(code, quiet)} fill="#000000" />
        </svg>
        <div className="share-side">
          <p className="subtle share-hint">
            {tu(
              "Mit der Kamera des Handys scannen. Der Text steht im Code selbst: kein Konto, kein Server, nichts wird hochgeladen. So kommt die Zusammenfassung aufs Handy oder zur Werkstatt.",
            )}
          </p>
          <pre className="share-text selectable">{text}</pre>
        </div>
      </div>
    </Modal>,
    document.body,
  );
}
