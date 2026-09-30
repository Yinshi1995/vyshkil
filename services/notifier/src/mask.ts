// Маскування номера (09-messaging.md §5: "Номери в логах маскуються (+380*****4567)") --
// лишає перші 4 і останні 4 символи, решту замінює зірочками. Викликається ПЕРЕД записом у
// `delivery_log`/логи -- повний номер ніколи не потрапляє далі `whatsapp/channel.ts`.

export function maskPhone(phone: string): string {
  if (phone.length <= 8) return "*".repeat(phone.length);
  const head = phone.slice(0, 4);
  const tail = phone.slice(-4);
  const middleStars = "*".repeat(phone.length - 8);
  return `${head}${middleStars}${tail}`;
}
