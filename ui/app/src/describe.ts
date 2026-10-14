import { skin } from "@app/skin-default";

export function describe(): string {
  return `the app: skin ${skin.id}, sdk ${skin.sdkVersion}`;
}
