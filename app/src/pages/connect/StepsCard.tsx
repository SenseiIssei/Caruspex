// The "how to" card of the connection page.
import { Card } from "../../components/ui";
import { translator } from "../../i18n";
import { Step } from "./parts";

const tr = translator("caruspex::ConnectPage");
// Texts of the truck option (SAE J1939).
const tk = translator("caruspex::Truck");

export function StepsCard({ socketHint, truck, motoSteps }: { socketHint: string; truck: boolean; motoSteps: string[] }) {
  return (
    <Card className="connect-steps" title={tr("So geht's")}>
      <Step n={1} icon="map-pin" title={tr("Buchse finden")} text={socketHint} />
      {truck ? (
        <div className="connect-moto">
          <div className="connect-step-head">{tk("Beim Lkw")}</div>
          <ul className="subtle">
            <li>
              {tk(
                "Nordamerikanische Lkw: runde 9-polige Diagnosebuchse (Deutsch) im Fahrerhaus, meist unter dem Armaturenbrett auf der Fahrerseite. Schwarz (Typ 1) heißt 250 kbit/s, grün (Typ 2, ab etwa 2016) 500 kbit/s.",
              )}
            </li>
            <li>{tk("Ein Adapterkabel von 9 Polen auf eine 16-polige OBD-Kupplung verbindet den Adapter.")}</li>
            <li>{tk("Europäische Lkw seit Euro 6 haben meist die 16-polige Buchse wie ein Auto.")}</li>
            <li>
              {tk(
                "Lkw haben meist 24 V. Viele billige ELM327-Adapter sind nur für 12 V gebaut und gehen daran kaputt: nur einen Adapter nehmen, der 24 V ausdrücklich verträgt.",
              )}
            </li>
          </ul>
        </div>
      ) : null}
      {motoSteps.length > 0 ? (
        <div className="connect-moto">
          <div className="connect-step-head">{tr("Beim Motorrad")}</div>
          <ul className="subtle">
            {motoSteps.map((line) => (
              <li key={line}>{line}</li>
            ))}
          </ul>
        </div>
      ) : null}
      <Step
        n={2}
        icon="cable"
        title={tr("Adapter einstecken")}
        text={tr(
          "ELM327 per USB, Bluetooth oder WLAN. Für Airbag, ABS und " +
            "Grundeinstellungen bei VAG bis etwa 2004 ein KKL-Kabel.",
        )}
      />
      <Step
        n={3}
        icon="key"
        title={tr("Zündung an")}
        text={tr(
          "Zündung einschalten. Der Motor darf aus sein, für Live-Werte " + "darf er laufen, aber nur im Freien.",
        )}
      />
      <Step
        n={4}
        icon="plug"
        title={tr("Verbinden")}
        text={tr(
          "Rechts auf Verbinden drücken. Beim ersten Mal dauert es ein " +
            "paar Sekunden, bis das Protokoll gefunden ist.",
        )}
      />
    </Card>
  );
}
