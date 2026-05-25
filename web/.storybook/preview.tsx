import type { Preview } from "@storybook/react";
import "../app/globals.css";

const preview: Preview = {
  parameters: {
    layout: "padded",
    backgrounds: {
      default: "skycot-light",
      values: [
        { name: "skycot-light", value: "#fdfcfb" },
        { name: "white", value: "#ffffff" },
      ],
    },
  },
};

export default preview;
