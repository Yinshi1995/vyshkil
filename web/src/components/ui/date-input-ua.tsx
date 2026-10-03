import { useState, useCallback, useEffect, useRef } from "react";
import { Input } from "./input";
import { fromIso, toIso, parseUaDate } from "@/lib/date-ua";

interface Props {
  value: string;
  onChange: (isoDate: string) => void;
  className?: string;
  placeholder?: string;
}

export function DateInputUa({ value, onChange, className, placeholder }: Props) {
  const [display, setDisplay] = useState(() => fromIso(value));
  const prevValue = useRef(value);

  useEffect(() => {
    if (value !== prevValue.current) {
      prevValue.current = value;
      setDisplay(fromIso(value));
    }
  }, [value]);
  const [error, setError] = useState(false);

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const raw = e.target.value;
      setDisplay(raw);

      if (!raw) {
        setError(false);
        onChange("");
        return;
      }

      const d = parseUaDate(raw);
      if (d) {
        setError(false);
        onChange(toIso(raw));
      } else {
        setError(raw.length >= 10);
      }
    },
    [onChange],
  );

  const handleBlur = useCallback(() => {
    if (display && !parseUaDate(display)) {
      setError(true);
    }
  }, [display]);

  return (
    <Input
      value={display}
      onChange={handleChange}
      onBlur={handleBlur}
      placeholder={placeholder ?? "ДД.ММ.РРРР"}
      className={className}
      style={error ? { borderColor: "var(--destructive)" } : undefined}
      maxLength={10}
    />
  );
}
