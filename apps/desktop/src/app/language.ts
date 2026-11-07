import { createContext, useContext } from "react";

export type Language = "es" | "en";

export const LanguageContext = createContext<Language>("es");

export function useLanguage() {
  const language = useContext(LanguageContext);
  return {
    language,
    t: (es: string, en: string) => (language === "en" ? en : es),
  };
}
