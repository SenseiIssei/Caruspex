// The dialogs of the garage: a service item, a service done, a fill-up.
import { useMemo, useState, type ReactNode } from "react";
import type { GarageService } from "../../api";
import { Modal } from "../../components/Modal";
import { Button, Check, NumberInput, Select, TextInput } from "../../components/ui";
import { translator } from "../../i18n";
import { addFuel, isoDate, markServiceDone, serviceTemplates, setService } from "../../state/garage";
import { cancelText } from "./shared";

const tr = translator("caruspex::GaragePage");

function DialogFooter({ onOk, onCancel, okDisabled }: { onOk: () => void; onCancel: () => void; okDisabled?: boolean }) {
  return (
    <>
      <Button onClick={onCancel}>{cancelText()}</Button>
      <Button variant="primary" onClick={onOk} disabled={okDisabled}>
        OK
      </Button>
    </>
  );
}

function FormRow({ label, children }: { label: ReactNode; children: ReactNode }) {
  return (
    <>
      <div className="form-label">{label}</div>
      <div className="garage-form-control">{children}</div>
    </>
  );
}

// "Wartung hinzufügen", also used to change an existing item.
export function ServiceDialog({ vin, odometer, edit, onClose }: { vin: string; odometer: number; edit: GarageService | null; onClose: () => void }) {
  const templates = useMemo(() => serviceTemplates(), []);
  const startIndex = edit ? templates.findIndex((s) => s.title === edit.title) : 0;
  const [kind, setKind] = useState(startIndex >= 0 ? startIndex : templates.length);
  const custom = kind >= templates.length;
  const [title, setTitle] = useState(edit ? edit.title : (templates[0]?.title ?? ""));
  const [km, setKm] = useState(edit ? edit.intervalKm : (templates[0]?.intervalKm ?? 0));
  const [months, setMonths] = useState(edit ? edit.intervalMonths : (templates[0]?.intervalMonths ?? 0));
  const [known, setKnown] = useState(edit ? !!edit.lastDate || edit.lastKm >= 0 : false);
  const [lastDate, setLastDate] = useState(edit?.lastDate || isoDate());
  const [lastKm, setLastKm] = useState(edit && edit.lastKm >= 0 ? edit.lastKm : odometer > 0 ? Math.round(odometer) : 0);

  const pickKind = (index: number) => {
    setKind(index);
    const tpl = templates[index];
    setTitle(tpl ? tpl.title : "");
    if (tpl) {
      setKm(tpl.intervalKm);
      setMonths(tpl.intervalMonths);
    }
  };

  const accept = () => {
    if (!title.trim()) {
      onClose();
      return;
    }
    setService(vin, {
      id: edit?.id ?? "",
      title: title.trim(),
      intervalKm: Math.max(0, Math.round(km)),
      intervalMonths: Math.max(0, Math.round(months)),
      lastDate: known ? lastDate : "",
      lastKm: known ? (lastKm > 0 ? lastKm : -1) : -1,
      note: edit?.note ?? "",
    });
    onClose();
  };

  return (
    <Modal
      small
      icon="wrench"
      title={edit ? tr("Wartung bearbeiten") : tr("Wartung hinzufügen")}
      onClose={onClose}
      footer={<DialogFooter onOk={accept} onCancel={onClose} />}
    >
      <div className="form garage-form">
        <FormRow label={tr("Art")}>
          <Select
            value={kind}
            options={[...templates.map((s, i) => ({ value: i, label: s.title })), { value: templates.length, label: tr("Eigener Eintrag") }]}
            onChange={pickKind}
          />
        </FormRow>
        <FormRow label={tr("Bezeichnung")}>
          <TextInput value={title} onChange={setTitle} disabled={!custom} />
        </FormRow>
        <FormRow label={tr("Alle")}>
          <NumberInput value={km} min={0} max={1000000} step={1000} suffix={km === 0 ? tr("nicht nach km") : "km"} onChange={setKm} />
        </FormRow>
        <FormRow label={tr("oder alle")}>
          <NumberInput
            value={months}
            min={0}
            max={240}
            suffix={months === 0 ? tr("nicht nach Zeit") : tr(" Monate").trim()}
            onChange={setMonths}
          />
        </FormRow>
        <FormRow
          label={
            <Check checked={known} onChange={setKnown}>
              {tr("Zuletzt erledigt am")}
            </Check>
          }
        >
          <input className="input" type="date" value={lastDate} onChange={(e) => e.target.value && setLastDate(e.target.value)} />
        </FormRow>
        <FormRow label={tr("bei Kilometerstand")}>
          <NumberInput value={lastKm} min={0} max={2000000} suffix={lastKm === 0 ? tr("unbekannt") : "km"} onChange={setLastKm} />
        </FormRow>
      </div>
    </Modal>
  );
}

export function DoneDialog({ vin, service, odometer, onClose }: { vin: string; service: GarageService; odometer: number; onClose: () => void }) {
  const [date, setDate] = useState(isoDate());
  const [km, setKm] = useState(odometer >= 0 ? Math.round(odometer) : 0);
  const accept = () => {
    markServiceDone(vin, service.id, date, km > 0 ? km : -1);
    onClose();
  };
  return (
    <Modal small icon="check" title={tr("%1 erledigt", service.title)} onClose={onClose} footer={<DialogFooter onOk={accept} onCancel={onClose} />}>
      <div className="form garage-form">
        <FormRow label={tr("Datum")}>
          <input className="input" type="date" value={date} onChange={(e) => e.target.value && setDate(e.target.value)} />
        </FormRow>
        <FormRow label={tr("Kilometerstand")}>
          <NumberInput value={km} min={0} max={2000000} suffix="km" onChange={setKm} />
        </FormRow>
      </div>
    </Modal>
  );
}

export function FuelDialog({ vin, odometer, onClose }: { vin: string; odometer: number; onClose: () => void }) {
  const [date, setDate] = useState(isoDate());
  const [km, setKm] = useState(odometer > 0 ? Math.round(odometer) : 0);
  const [litres, setLitres] = useState(0);
  const [price, setPrice] = useState(1.799);
  const [full, setFull] = useState(true);
  const accept = () => {
    if (litres > 0) {
      addFuel(vin, { date, odometerKm: km > 0 ? km : -1, litres, pricePerLitre: price, full });
    }
    onClose();
  };
  return (
    <Modal small icon="fuel" title={tr("Tankfüllung eintragen")} onClose={onClose} footer={<DialogFooter onOk={accept} onCancel={onClose} />}>
      <div className="form garage-form">
        <FormRow label={tr("Datum")}>
          <input className="input" type="date" value={date} onChange={(e) => e.target.value && setDate(e.target.value)} />
        </FormRow>
        <FormRow label={tr("Kilometerstand")}>
          <NumberInput value={km} min={0} max={2000000} suffix="km" onChange={setKm} />
        </FormRow>
        <FormRow label={tr("Menge")}>
          <NumberInput value={litres} min={0} max={500} step={0.01} decimals={2} suffix="l" onChange={setLitres} />
        </FormRow>
        <FormRow label={tr("Preis")}>
          <NumberInput value={price} min={0} max={10} step={0.001} decimals={3} suffix="€/l" onChange={setPrice} />
        </FormRow>
        <div className="full">
          <Check checked={full} onChange={setFull}>
            {tr("voll getankt")}
          </Check>
        </div>
      </div>
    </Modal>
  );
}
