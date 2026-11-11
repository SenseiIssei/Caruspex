// Settings (SettingsPage.cpp). Every knob here actually has an effect;
// anything that only looks like a choice but doesn't reach anything does not
// belong on this page. Every change is applied at once, like the Qt
// "changed" signal, so nothing waits for a save button.
import { useEffect, useState, type ReactNode } from "react";
import { call } from "../api";
import type { AppSettings } from "../api";
import { DEFAULT_SETTINGS } from "../api";
import { Button, Card, Check, NumberInput, Page, PageHeader, Segmented, Select, TextInput } from "../components/ui";
import { LANGUAGES, translator } from "../i18n";
import { setAppearance, type Appearance } from "../lib/theme";
import { applySettings, setProMode, useProMode, useSettings } from "../state/session";
import "./SettingsPage.css";

const tr = translator("caruspex::SettingsPage");

function Group({ title, children, glow }: { title: string; children: ReactNode; glow?: boolean }) {
  return (
    <Card className={glow ? "settings-group glow" : "settings-group"} title={title}>
      <div className="form settings-form">{children}</div>
    </Card>
  );
}

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <>
      <label className="form-label">{label}</label>
      <div className="settings-control">{children}</div>
    </>
  );
}

function Hint({ children }: { children: ReactNode }) {
  return <div className="full hint settings-note">{children}</div>;
}

function Full({ children }: { children: ReactNode }) {
  return <div className="full">{children}</div>;
}

// A spin box for one setting. It keeps what is being typed until the value
// is inside the range, so "150" can be typed over "100" digit by digit;
// leaving the field clamps it, like QSpinBox.
function SettingNumber({
  value,
  min,
  max,
  step,
  suffix,
  onChange,
}: {
  value: number;
  min: number;
  max: number;
  step?: number;
  suffix?: string;
  onChange: (value: number) => void;
}) {
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value]);
  return (
    <NumberInput
      value={draft}
      min={min}
      max={max}
      step={step}
      suffix={suffix}
      onChange={(v) => {
        setDraft(v);
        const rounded = Math.round(v);
        if (rounded >= min && rounded <= max && rounded !== value) onChange(rounded);
      }}
    />
  );
}

export default function SettingsPage() {
  const values = useSettings();
  const pro = useProMode();
  // The path field reports on leaving it (editingFinished), not per key.
  const [logPath, setLogPath] = useState(values.logPath);
  useEffect(() => setLogPath(values.logPath), [values.logPath]);

  const change = (patch: Partial<AppSettings>) => {
    const next = { ...values, ...patch };
    // The look follows at once.
    if (patch.appearance !== undefined) setAppearance(next.appearance as Appearance);
    void applySettings(next);
  };

  // The last port is no setting of this page; it stays.
  const reset = () => {
    const defaults = { ...DEFAULT_SETTINGS, lastPort: values.lastPort, proMode: values.proMode };
    setAppearance(defaults.appearance as Appearance);
    void applySettings(defaults);
  };

  const chooseLogPath = async () => {
    const path = await call("pick_save_path", {
      title: tr("Protokolldatei wählen"),
      suggestedName: values.logPath || "caruspex.log",
      filters: [{ name: tr("Protokolldatei (*.log *.txt)"), extensions: ["log", "txt"] }],
    });
    if (!path) return;
    setLogPath(path);
    change({ logPath: path });
  };

  const interval = values.pollIntervalMs;
  const pollHint =
    interval === 0
      ? tr("Null heißt: so schnell der Adapter mitkommt. Auf der K-Line ist das ohnehin durch das Protokoll begrenzt.")
      : tr(
          "Etwa %1 Abfragen je Sekunde. Ein sehr kleiner Wert schüttet die Oberfläche mit Signalen zu, ein großer lässt die Diagramme stocken.",
          Math.trunc(1000 / interval),
        );

  const baudOptions = [
    { value: 0, label: tr("Automatisch durchprobieren") },
    ...[9600, 19200, 38400, 57600, 115200, 230400, 500000].map((b) => ({ value: b, label: tr("%1 Baud", b) })),
  ];
  const orderOptions = [
    { value: "kline", label: tr("K-Line zuerst (A2 und ältere)") },
    { value: "can", label: tr("CAN zuerst (A4 B7 und neuere)") },
  ];
  const timingOptions = [
    { value: 0, label: tr("Standard (ATAT1)") },
    { value: 1, label: tr("Schneller bei OBDLink und anderen STN-Adaptern") },
    { value: 2, label: tr("Immer schneller (ATAT2)") },
  ];
  const ecuOptions = [
    { value: 0x01, label: tr("01  Motor") },
    { value: 0x02, label: tr("02  Getriebe") },
    { value: 0x03, label: tr("03  Bremsenelektronik (ABS)") },
    { value: 0x08, label: tr("08  Klimaanlage") },
    { value: 0x09, label: tr("09  Zentralelektrik") },
    { value: 0x15, label: tr("15  Airbag") },
    { value: 0x16, label: tr("16  Lenkradelektronik") },
    { value: 0x17, label: tr("17  Kombiinstrument") },
    { value: 0x25, label: tr("25  Wegfahrsperre") },
    { value: 0x46, label: tr("46  Komfortsystem") },
    { value: 0x56, label: tr("56  Radio") },
  ];
  const sessionOptions = [
    { value: 0x85, label: tr("0x85  Grundeinstellung (üblich)") },
    { value: 0x86, label: tr("0x86  Anpassung") },
    { value: 0x87, label: tr("0x87  Erweiterte Diagnose") },
    { value: 0x89, label: tr("0x89  Werkseinstellung") },
  ];
  // Every language under its own name, so anyone finds theirs.
  const languageOptions = [
    { value: "auto", label: tr("Wie das System") },
    ...LANGUAGES.map((l) => ({ value: l.code, label: l.nativeName })),
  ];
  const appearanceOptions = [
    { value: "auto", label: tr("Wie das System") },
    { value: "dark", label: tr("Dunkel") },
    { value: "light", label: tr("Hell") },
  ];

  // Values the lists do not know fall back to their first entry, like
  // findData() returning -1 in showValues().
  const pick = <T extends string | number>(options: { value: T }[], value: T): T =>
    options.some((o) => o.value === value) ? value : options[0]!.value;

  return (
    <Page className="settings-page">
      <PageHeader
        eyebrow={tr("PROGRAMM")}
        title={tr("Einstellungen")}
        subtitle={tr(
          "Alles wird sofort übernommen und beim nächsten Start wieder so geladen. Änderungen wirken auch während einer laufenden Messung.",
        )}
      />

      <Group title={tr("Bedienung")} glow>
        <Full>
          <Segmented
            value={pro ? "pro" : "beginner"}
            options={[
              { value: "beginner", label: tr("Einsteiger"), icon: "lightbulb" },
              { value: "pro", label: tr("Profi"), icon: "wrench" },
            ]}
            onChange={(v) => setProMode(v === "pro")}
          />
        </Full>
        <Hint>
          {pro
            ? tr(
                "Profis sehen alles: Live-Werte, Messungen, alle Steuergeräte, KKL, Anpassung und Stellglieder, Drosselklappe, Rohbefehle und Adressen.",
              )
            : tr(
                "Einsteiger sehen nur die Seiten für den Alltag. Live-Werte, Messungen, alle Steuergeräte, KKL, Anpassung, Drosselklappe, Rohbefehle und das Wissen kommen mit dem Profimodus.",
              )}
        </Hint>
      </Group>

      <Group title={tr("Verbindung")}>
        <Row label={tr("Baudrate des Adapters")}>
          <Select value={pick(baudOptions, values.baudOverride)} options={baudOptions} onChange={(v) => change({ baudOverride: v })} />
        </Row>
        <Row label={tr("Reihenfolge der Protokolle")}>
          <Select value={pick(orderOptions, values.protocolOrder)} options={orderOptions} onChange={(v) => change({ protocolOrder: v })} />
        </Row>
        <Row label={tr("Geduld mit dem Adapter")}>
          <SettingNumber
            value={values.timeoutPercent}
            min={25}
            max={500}
            step={25}
            suffix={tr(" %").trim()}
            onChange={(v) => change({ timeoutPercent: v })}
          />
        </Row>
        <Row label={tr("Taktung")}>
          <Select
            value={pick(timingOptions, Math.min(2, Math.max(0, values.adaptiveTiming)))}
            options={timingOptions}
            onChange={(v) => change({ adaptiveTiming: v })}
          />
        </Row>
        <Hint>
          {tr(
            "Streckt oder kürzt jede Wartezeit. Ein träger Klon braucht mehr, ein gutes FTDI-Kabel kommt mit weniger aus. Die Abstufung bleibt erhalten: Löschen darf länger dauern als eine einzelne Abfrage.",
          )}
        </Hint>
        <Full>
          <Check checked={values.autoConnect} onChange={(v) => change({ autoConnect: v })}>
            {tr("Beim Start gleich mit dem zuletzt benutzten Anschluss verbinden")}
          </Check>
        </Full>
      </Group>

      <Group title={tr("Messwerte")}>
        <Row label={tr("Abstand zwischen zwei Abfragen")}>
          <SettingNumber
            value={values.pollIntervalMs}
            min={0}
            max={5000}
            step={10}
            suffix={tr(" ms").trim()}
            onChange={(v) => change({ pollIntervalMs: v })}
          />
        </Row>
        <Hint>{pollHint}</Hint>
        <Row label={tr("Zeitfenster der Diagramme")}>
          <SettingNumber
            value={values.chartWindowSeconds}
            min={10}
            max={3600}
            step={10}
            suffix={tr(" s").trim()}
            onChange={(v) => change({ chartWindowSeconds: v })}
          />
        </Row>
      </Group>

      <Group title={tr("Fehler löschen")}>
        <Row label={tr("Versuche beim hartnäckigen Löschen")}>
          <SettingNumber
            value={values.clearAttempts}
            min={1}
            max={20}
            onChange={(v) => change({ clearAttempts: v })}
          />
        </Row>
        <Hint>
          {tr(
            "Nach jedem Versuch wird nachgelesen. Bleibt ein Fehler stehen, liegt er meist noch an: dann hilft kein weiterer Versuch, sondern nur die Reparatur.",
          )}
        </Hint>
        <Full>
          <Check checked={values.confirmClear} onChange={(v) => change({ confirmClear: v })}>
            {tr("Vor dem Löschen nachfragen")}
          </Check>
        </Full>
      </Group>

      {pro ? (
        <Group title={tr("K-Line und KKL-Kabel")}>
          <Row label={tr("Voreingestelltes Steuergerät")}>
            <Select value={pick(ecuOptions, values.kklEcuAddress)} options={ecuOptions} onChange={(v) => change({ kklEcuAddress: v })} />
          </Row>
          <Row label={tr("Sitzungsart bei KWP2000")}>
            <Select
              value={pick(sessionOptions, values.basicSettingSession)}
              options={sessionOptions}
              onChange={(v) => change({ basicSettingSession: v })}
            />
          </Row>
          <Hint>
            {tr(
              "Welche Sitzungsart der 1.6 FSI für die Grundeinstellung verlangt, ist nicht öffentlich dokumentiert. Deshalb steht sie hier zur Wahl. Lehnt das Steuergerät ab, ist die nächste einen Versuch wert. Nur den KWP1281-Weg betrifft das nicht, dort gibt es diese Wahl nicht.",
            )}
          </Hint>
          <Row label={tr("Nachfragetakt in der Grundeinstellung")}>
            <SettingNumber
              value={values.basicSettingIntervalMs}
              min={50}
              max={2000}
              step={10}
              suffix={tr(" ms").trim()}
              onChange={(v) => change({ basicSettingIntervalMs: v })}
            />
          </Row>
          <Hint>
            {tr(
              "Das Steuergerät fällt aus der Grundeinstellung, wenn länger als etwa eine Sekunde nichts kommt. Deutlich höher als 500 ms zu gehen ist riskant, während die Drosselklappe lernt.",
            )}
          </Hint>
        </Group>
      ) : null}

      <Group title={tr("Protokoll")}>
        <Full>
          <Check checked={values.logRaw} onChange={(v) => change({ logRaw: v })}>
            {tr("Rohkommunikation mitschreiben (jede Zeile hin und zurück)")}
          </Check>
        </Full>
        <Full>
          <Check checked={values.logToFile} onChange={(v) => change({ logToFile: v })}>
            {tr("Zusätzlich in eine Datei schreiben")}
          </Check>
        </Full>
        <Row label={tr("Datei")}>
          <div className="settings-path">
            <TextInput
              className="grow"
              value={logPath}
              onChange={setLogPath}
              onBlur={() => logPath.trim() !== values.logPath && change({ logPath: logPath.trim() })}
              onKeyDown={(e) => e.key === "Enter" && (e.target as HTMLInputElement).blur()}
              placeholder={tr("noch keine Datei gewählt")}
              disabled={!values.logToFile}
              spellCheck={false}
            />
            <Button onClick={() => void chooseLogPath()} disabled={!values.logToFile}>
              {tr("Wählen ...")}
            </Button>
          </div>
        </Row>
      </Group>

      <Group title={tr("Fenster und Sprache")}>
        <Full>
          <Check checked={values.rememberGeometry} onChange={(v) => change({ rememberGeometry: v })}>
            {tr("Größe und Position beim Beenden merken")}
          </Check>
        </Full>
        <Full>
          <Check checked={values.checkUpdates} onChange={(v) => change({ checkUpdates: v })}>
            {tr("Beim Start nach einer neuen Version suchen (fragt nur GitHub, sonst nichts)")}
          </Check>
        </Full>
        <Row label={tr("Sprache")}>
          <Select value={pick(languageOptions, values.language)} options={languageOptions} onChange={(v) => change({ language: v })} />
        </Row>
        <Row label={tr("Erscheinungsbild")}>
          <Select
            value={pick(appearanceOptions, values.appearance)}
            options={appearanceOptions}
            onChange={(v) => change({ appearance: v })}
          />
        </Row>
        <Hint>{tr("Sprache und Erscheinungsbild wechseln beim nächsten Start.")}</Hint>
      </Group>

      <Group title={tr("Lizenzen und Datenquellen")}>
        <Hint>{tr("Caruspex ist freie Software unter der MIT-Lizenz.")}</Hint>
        <Hint>
          {tr(
            "Die Herstellerwerte für viele Modelle stammen aus OBDb (github.com/OBDb) und stehen unter der Lizenz CC BY-SA 4.0 (Namensnennung, Weitergabe unter gleichen Bedingungen). Die umgewandelten Daten liegen mit Lizenz und Liste der Quellen im Ordner data/obdb des Quellcodes.",
          )}
        </Hint>
      </Group>

      <div className="row wrap-row">
        <Button icon="refresh" onClick={reset}>
          {tr("Auf Vorgabe zurücksetzen")}
        </Button>
        <Button icon="file" onClick={() => void call("open_config_location")}>
          {tr("Ablageort öffnen")}
        </Button>
      </div>
    </Page>
  );
}
