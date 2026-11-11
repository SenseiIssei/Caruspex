// A small hint for beginners where a Profi page would help: what it does,
// and one button that switches the Profi mode on and opens the page.
// Shows nothing in Profi mode.
import { translator } from "../i18n";
import { navigate } from "../shell/nav";
import type { PageId } from "../shell/pages";
import { setProMode, useProMode } from "../state/session";
import { Icon } from "./Icon";
import { Button } from "./ui";

const tr = translator("caruspex::WhatToDoPage");

export function ProHint({ text, page }: { text: string; page?: PageId }) {
  const pro = useProMode();
  if (pro) return null;
  return (
    <div className="pro-hint">
      <Icon name="lightbulb" size={16} className="pro-hint-icon" />
      <span className="grow">{text}</span>
      <Button
        small
        variant="ghost"
        icon="sliders"
        onClick={() => {
          setProMode(true);
          if (page) navigate(page);
        }}
      >
        {tr("Profimodus einschalten")}
      </Button>
    </div>
  );
}
