// Where the interface runs, and the phone layout. The Android app is this
// page in the system's WebView; its user agent says so before any command
// has answered. The phone layout follows the width only, so a narrow
// desktop window shows it too.
import { useEffect, useState } from "react";

export const isAndroid: boolean = typeof navigator !== "undefined" && /Android/i.test(navigator.userAgent);

// Up to this width the sidebar becomes a menu that slides in.
export const PHONE_QUERY = "(max-width: 640px)";

function phoneNow(): boolean {
  return typeof window !== "undefined" && typeof window.matchMedia === "function" && window.matchMedia(PHONE_QUERY).matches;
}

export function usePhoneLayout(): boolean {
  const [phone, setPhone] = useState(phoneNow);
  useEffect(() => {
    if (typeof window.matchMedia !== "function") return;
    const query = window.matchMedia(PHONE_QUERY);
    const update = () => setPhone(query.matches);
    update();
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  }, []);
  return phone;
}
