import i18n from "i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import { initReactI18next } from "react-i18next";

import en from "./locales/en.json";
import zhTW from "./locales/zh-TW.json";
import zhCN from "./locales/zh-CN.json";

export const SUPPORTED_LANGUAGES = ["en", "zh-TW", "zh-CN"] as const;
export type SupportedLanguage = (typeof SUPPORTED_LANGUAGES)[number];

void i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      en: { translation: en },
      "zh-TW": { translation: zhTW },
      "zh-CN": { translation: zhCN },
    },
    // Strict matching against our supported set; anything else falls back to
    // zh-TW (e.g. Japanese or French OS locale → Traditional Chinese).
    supportedLngs: ["en", "zh-TW", "zh-CN"],
    fallbackLng: "zh-TW",
    nonExplicitSupportedLngs: false,
    interpolation: { escapeValue: false },
    detection: {
      // Honor a user-saved choice over the OS locale, but default to the OS
      // when no choice has been made.
      order: ["localStorage", "navigator", "htmlTag"],
      lookupLocalStorage: "yande-dl.lang",
      caches: ["localStorage"],
    },
  });

export default i18n;
