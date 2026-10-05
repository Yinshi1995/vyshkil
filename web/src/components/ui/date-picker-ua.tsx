import { useState } from "react";
import { Calendar } from "./calendar";
import { Popover, PopoverContent, PopoverTrigger } from "./popover";
import { Button } from "./button";
import { CalendarIcon } from "lucide-react";
import { format, parse } from "date-fns";
import { uk } from "date-fns/locale";

interface Props {
  value: string;
  onChange: (isoDate: string) => void;
  className?: string;
  placeholder?: string;
}

export function DatePickerUa({ value, onChange, className, placeholder }: Props) {
  const [open, setOpen] = useState(false);

  const selected = value ? parse(value, "yyyy-MM-dd", new Date()) : undefined;

  return (
    <Popover open={open} onOpenChange={setOpen}>
      <PopoverTrigger
        render={
          <Button
            variant="outline"
            className={`justify-start text-left font-normal ${className ?? ""}`}
          />
        }
      >
        <CalendarIcon className="mr-2 h-4 w-4" />
        {selected ? (
          format(selected, "dd.MM.yyyy")
        ) : (
          <span className="text-muted-foreground">
            {placeholder ?? "Оберіть дату"}
          </span>
        )}
      </PopoverTrigger>
      <PopoverContent className="w-auto p-0" align="start">
        <Calendar
          mode="single"
          locale={uk}
          selected={selected}
          onSelect={(date) => {
            if (date) {
              onChange(format(date, "yyyy-MM-dd"));
            } else {
              onChange("");
            }
            setOpen(false);
          }}
        />
      </PopoverContent>
    </Popover>
  );
}
