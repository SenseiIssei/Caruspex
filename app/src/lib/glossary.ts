// The technical words Caruspex uses, each with a one-sentence hint (shown
// at the word itself, see components/Term.tsx) and a longer explanation for
// the glossary page under Wissen. German is the source, like every
// interface text; the translations live in i18n/new-texts/usability2.json.
import { translator } from "../i18n";

const tg = translator("caruspex::Glossary");

export type TermId =
  | "dtc"
  | "dtc-stored"
  | "dtc-pending"
  | "dtc-permanent"
  | "mil"
  | "readiness"
  | "freeze-frame"
  | "stft"
  | "ltft"
  | "pid"
  | "ecu"
  | "obd"
  | "vin"
  | "uds"
  | "kwp"
  | "can"
  | "elm327"
  | "kkl";

export interface GlossaryEntry {
  id: TermId;
  // The word as it appears in the interface.
  term: () => string;
  // One sentence, for the hint at the word.
  short: () => string;
  // Two or three sentences for the glossary page: what it means for the car.
  long: () => string;
}

export const GLOSSARY: readonly GlossaryEntry[] = [
  {
    id: "dtc",
    term: () => tg("Fehlercode (DTC)"),
    short: () => tg("Ein Code wie P0171, den ein Steuergerät speichert, wenn es etwas Auffälliges bemerkt."),
    long: () =>
      tg(
        "Der erste Buchstabe nennt den Bereich: P Antrieb, B Karosserie, C Fahrwerk, U Vernetzung. Eine 0 an zweiter Stelle heißt genormt, eine 1 herstellereigen. Der Code sagt, wo etwas auffiel, nicht welches Teil kaputt ist.",
      ),
  },
  {
    id: "dtc-stored",
    term: () => tg("Gespeicherter Fehler"),
    short: () => tg("Ein bestätigter Fehler: er trat mehrmals auf und schaltet oft die Motorkontrollleuchte ein."),
    long: () =>
      tg(
        "Das Steuergerät hat den Fehler über mehrere Fahrten gesehen und für echt befunden. Er bleibt im Speicher, bis er gelöscht wird oder das Auto ihn nach vielen fehlerfreien Fahrten selbst vergisst.",
      ),
  },
  {
    id: "dtc-pending",
    term: () => tg("Sporadischer Fehler"),
    short: () => tg("Ein vorläufiger Fehler: einmal aufgefallen, aber noch nicht bestätigt."),
    long: () =>
      tg(
        "Tritt er bei der nächsten Fahrt wieder auf, wird daraus ein gespeicherter Fehler. Ein vorläufiger Fehler ist eine frühe Warnung und oft der beste Hinweis, bevor die Lampe angeht.",
      ),
  },
  {
    id: "dtc-permanent",
    term: () => tg("Dauerhafter Fehler"),
    short: () => tg("Ein Fehler, den kein Tester löschen kann; das Auto löscht ihn selbst, wenn die Prüfung wieder besteht."),
    long: () =>
      tg(
        "Seit etwa 2010 merken sich viele Autos Abgasfehler zusätzlich so. Er verschwindet erst, wenn die Ursache behoben ist und das Steuergerät die Prüfung bei einer Fahrt erneut bestanden hat. So lässt sich ein Fehler vor der Abgasuntersuchung nicht einfach verstecken.",
      ),
  },
  {
    id: "mil",
    term: () => tg("Motorkontrollleuchte (MIL)"),
    short: () => tg("Die gelbe Motorlampe im Kombiinstrument; sie leuchtet bei einem bestätigten Abgasfehler."),
    long: () =>
      tg(
        "Leuchtet sie dauerhaft, bald in die Werkstatt. Blinkt sie, setzen Zündaussetzer aus, die den Katalysator zerstören können: langsam fahren und bald anhalten.",
      ),
  },
  {
    id: "readiness",
    term: () => tg("Bereitschaft (Readiness)"),
    short: () => tg("Zeigt, ob das Auto seine eigenen Abgasprüfungen seit dem letzten Löschen abgeschlossen hat."),
    long: () =>
      tg(
        "Jedes Teilsystem, etwa Katalysator, Lambdasonden oder Tankentlüftung, prüft sich bei passenden Fahrten selbst. Nach dem Löschen oder Abklemmen der Batterie sind alle wieder offen. Für die Abgasuntersuchung müssen fast alle fertig sein; sind viele offen, wurde vielleicht kurz vorher gelöscht.",
      ),
  },
  {
    id: "freeze-frame",
    term: () => tg("Freeze Frame"),
    short: () => tg("Eine Momentaufnahme der Messwerte in dem Augenblick, in dem der Fehler gespeichert wurde."),
    long: () =>
      tg(
        "Drehzahl, Temperatur, Geschwindigkeit und Last zeigen, unter welchen Umständen der Fehler auftrat: kalt oder warm, im Leerlauf oder unter Last. Das hilft, ihn in der Werkstatt nachzustellen.",
      ),
  },
  {
    id: "stft",
    term: () => tg("Kurzzeitkorrektur (STFT)"),
    short: () => tg("Wie stark das Steuergerät gerade jetzt Kraftstoff zugibt oder wegnimmt, in Prozent."),
    long: () =>
      tg(
        "Der Wert springt ständig um null, das ist normal: so hält die Lambdaregelung das Gemisch. Plus heißt mehr Kraftstoff, minus weniger.",
      ),
  },
  {
    id: "ltft",
    term: () => tg("Langzeitkorrektur (LTFT)"),
    short: () => tg("Die gelernte Gemischkorrektur über viele Fahrten; mehr als etwa 10 % in eine Richtung deutet auf einen Fehler."),
    long: () =>
      tg(
        "Hohe Pluswerte sprechen für Falschluft, einen schwachen Luftmassenmesser oder zu wenig Kraftstoffdruck. Hohe Minuswerte für zu viel Kraftstoff, etwa eine undichte Einspritzdüse. Die Korrektur wird beim Löschen der Fehler zurückgesetzt.",
      ),
  },
  {
    id: "pid",
    term: () => tg("PID"),
    short: () => tg("Die Nummer eines Messwerts, den ein Steuergerät auf Anfrage liefert, etwa 0C für die Drehzahl."),
    long: () =>
      tg(
        "Die genormten PIDs kennt jedes Auto mit OBD-II, aber nicht jedes Auto liefert alle. Hersteller haben eigene Nummern für weitere Werte, etwa den Akku eines E-Autos.",
      ),
  },
  {
    id: "ecu",
    term: () => tg("Steuergerät (ECU)"),
    short: () => tg("Ein kleiner Rechner im Auto, der einen Bereich regelt, etwa Motor, Getriebe, ABS oder Airbag."),
    long: () =>
      tg(
        "Moderne Autos haben zwanzig bis über hundert davon, verbunden über Datenleitungen. Jedes führt seinen eigenen Fehlerspeicher; die genormte OBD-Abfrage erreicht meist nur die für das Abgas wichtigen.",
      ),
  },
  {
    id: "obd",
    term: () => tg("OBD-II und EOBD"),
    short: () => tg("Die genormte Diagnose an der 16-poligen Buchse: Pflicht in Europa für Benziner ab 2001 und Diesel ab 2004."),
    long: () =>
      tg(
        "In den USA gilt sie ab 1996. Sie deckt vor allem ab, was das Abgas betrifft; Airbag, ABS und Komfort sprechen herstellereigene Protokolle.",
      ),
  },
  {
    id: "vin",
    term: () => tg("Fahrgestellnummer (VIN)"),
    short: () => tg("Die 17 Zeichen, die jedes Auto eindeutig kennzeichnen; Hersteller, Modell und Baujahr stecken darin."),
    long: () =>
      tg(
        "Sie steht im Fahrzeugschein, unten an der Windschutzscheibe und im Motorsteuergerät. Caruspex liest sie aus und erkennt daran das Auto.",
      ),
  },
  {
    id: "uds",
    term: () => tg("UDS"),
    short: () => tg("Das Diagnoseprotokoll der neueren Autos (ISO 14229), über CAN oder Ethernet."),
    long: () =>
      tg(
        "Darüber erreicht Caruspex auch Steuergeräte jenseits des Motors und liest deren Fehlerspeicher und Kennungen. Es kennt Sitzungen und Sicherheitszugänge; Caruspex nutzt nur das Lesen.",
      ),
  },
  {
    id: "kwp",
    term: () => tg("KWP1281 und KWP2000"),
    short: () => tg("Ältere Diagnoseprotokolle über die K-Line, bei VW, Audi, Seat und Skoda bis etwa 2005 verbreitet."),
    long: () =>
      tg(
        "KWP1281 ist das ganz alte VAG-Protokoll, KWP2000 (ISO 14230) sein Nachfolger. Über sie erreicht ein KKL-Kabel Airbag, ABS, Klima und weitere Steuergeräte, die die genormte OBD-Abfrage nicht zeigt.",
      ),
  },
  {
    id: "can",
    term: () => tg("CAN-Bus"),
    short: () => tg("Die Datenleitung, über die sich die Steuergeräte der meisten Autos ab etwa 2005 unterhalten."),
    long: () =>
      tg(
        "An der OBD-Buchse liegt er auf den Stiften 6 und 14, meist mit 500 kbit/s. Viele Busse im Auto liegen hinter einem Gateway und sind von der Buchse aus nicht zu hören.",
      ),
  },
  {
    id: "elm327",
    term: () => tg("ELM327"),
    short: () => tg("Der verbreitete Chip in günstigen OBD-Adaptern; er übersetzt zwischen Rechner und Auto."),
    long: () =>
      tg(
        "Viele billige Adapter tragen einen Nachbau, der nicht alle Befehle kann. Caruspex prüft den Adapter auf der Seite Adapter und sagt, was er beherrscht.",
      ),
  },
  {
    id: "kkl",
    term: () => tg("KKL und K-Line"),
    short: () => tg("Die ältere Ein-Draht-Datenleitung (Stift 7) und das passende USB-Kabel dafür."),
    long: () =>
      tg(
        "Ein KKL-Kabel spricht direkt mit den Steuergeräten älterer Autos, vor allem bei VW, Audi, Seat und Skoda. Am besten eines mit FTDI-Chip; mit dem ELM327 geht das nur eingeschränkt.",
      ),
  },
];

export function glossaryEntry(id: TermId): GlossaryEntry | undefined {
  return GLOSSARY.find((e) => e.id === id);
}
